use std::collections::{HashMap, BTreeSet, HashSet};
use std::fs;
use std::path::{PathBuf, Path};
use std::str;
use std::sync::{Mutex, Arc};

use core::PackageId;
use util::{CargoResult, Human, Freshness, Cfg};
use util::{internal, ChainError, profile, paths};
use util::machine_message;

use super::job::Work;
use super::{fingerprint, Kind, Context, Unit};

/// Contains the parsed output of a custom build script.
#[derive(Clone, Debug, Hash)]
pub struct BuildOutput {
    /// Paths to pass to rustc with the `-L` flag
    pub library_paths: Vec<PathBuf>,
    /// Names and link kinds of libraries, suitable for the `-l` flag
    pub library_links: Vec<String>,
    /// Various `--cfg` flags to pass to the compiler
    pub cfgs: Vec<String>,
    /// Metadata to pass to the immediate dependencies
    pub metadata: Vec<(String, String)>,
    /// Paths to trigger a rerun of this build script.
    pub rerun_if_changed: Vec<String>,
    /// Warnings generated by this build,
    pub warnings: Vec<String>,
}

pub type BuildMap = HashMap<(PackageId, Kind), BuildOutput>;

pub struct BuildState {
    pub outputs: Mutex<BuildMap>,
    overrides: HashMap<(String, Kind), BuildOutput>,
}

#[derive(Default)]
pub struct BuildScripts {
    // Cargo will use this `to_link` vector to add -L flags to compiles as we
    // propagate them upwards towards the final build. Note, however, that we
    // need to preserve the ordering of `to_link` to be topologically sorted.
    // This will ensure that build scripts which print their paths properly will
    // correctly pick up the files they generated (if there are duplicates
    // elsewhere).
    //
    // To preserve this ordering, the (id, kind) is stored in two places, once
    // in the `Vec` and once in `seen_to_link` for a fast lookup. We maintain
    // this as we're building interactively below to ensure that the memory
    // usage here doesn't blow up too much.
    //
    // For more information, see #2354
    pub to_link: Vec<(PackageId, Kind)>,
    seen_to_link: HashSet<(PackageId, Kind)>,
    pub plugins: BTreeSet<PackageId>,
}

/// Prepares a `Work` that executes the target as a custom build script.
///
/// The `req` given is the requirement which this run of the build script will
/// prepare work for. If the requirement is specified as both the target and the
/// host platforms it is assumed that the two are equal and the build script is
/// only run once (not twice).
pub fn prepare<'a, 'cfg>(cx: &mut Context<'a, 'cfg>, unit: &Unit<'a>)
                         -> CargoResult<(Work, Work, Freshness)> {
    let _p = profile::start(format!("build script prepare: {}/{}",
                                    unit.pkg, unit.target.name()));
    let overridden = cx.build_state.has_override(unit);
    let (work_dirty, work_fresh) = if overridden {
        (Work::new(|_| Ok(())), Work::new(|_| Ok(())))
    } else {
        build_work(cx, unit)?
    };

    // Now that we've prep'd our work, build the work needed to manage the
    // fingerprint and then start returning that upwards.
    let (freshness, dirty, fresh) =
            fingerprint::prepare_build_cmd(cx, unit)?;

    Ok((work_dirty.then(dirty), work_fresh.then(fresh), freshness))
}

fn build_work<'a, 'cfg>(cx: &mut Context<'a, 'cfg>, unit: &Unit<'a>)
                        -> CargoResult<(Work, Work)> {
    let dependencies = cx.dep_run_custom_build(unit)?;
    let build_script_unit = dependencies.iter().find(|d| {
        !d.profile.run_custom_build && d.target.is_custom_build()
    }).expect("running a script not depending on an actual script");
    let script_output = cx.build_script_dir(build_script_unit);
    let build_output = cx.build_script_out_dir(unit);

    // Building the command to execute
    let to_exec = script_output.join(unit.target.name());

    // Start preparing the process to execute, starting out with some
    // environment variables. Note that the profile-related environment
    // variables are not set with this the build script's profile but rather the
    // package's library profile.
    let profile = cx.lib_profile();
    let to_exec = to_exec.into_os_string();
    let mut cmd = cx.compilation.host_process(to_exec, unit.pkg)?;
    cmd.env("OUT_DIR", &build_output)
       .env("CARGO_MANIFEST_DIR", unit.pkg.root())
       .env("NUM_JOBS", &cx.jobs().to_string())
       .env("TARGET", &match unit.kind {
           Kind::Host => cx.host_triple(),
           Kind::Target => cx.target_triple(),
       })
       .env("DEBUG", &profile.debuginfo.is_some().to_string())
       .env("OPT_LEVEL", &profile.opt_level)
       .env("PROFILE", if cx.build_config.release { "release" } else { "debug" })
       .env("HOST", cx.host_triple())
       .env("RUSTC", &cx.config.rustc()?.path)
       .env("RUSTDOC", &*cx.config.rustdoc()?);

    if let Some(links) = unit.pkg.manifest().links() {
        cmd.env("CARGO_MANIFEST_LINKS", links);
    }

    // Be sure to pass along all enabled features for this package, this is the
    // last piece of statically known information that we have.
    if let Some(features) = cx.resolve.features(unit.pkg.package_id()) {
        for feat in features.iter() {
            cmd.env(&format!("CARGO_FEATURE_{}", super::envify(feat)), "1");
        }
    }

    let mut cfg_map = HashMap::new();
    for cfg in cx.cfg(unit.kind) {
        match *cfg {
            Cfg::Name(ref n) => { cfg_map.insert(n.clone(), None); }
            Cfg::KeyPair(ref k, ref v) => {
                match *cfg_map.entry(k.clone()).or_insert(Some(Vec::new())) {
                    Some(ref mut values) => values.push(v.clone()),
                    None => { /* ... */ }
                }
            }
        }
    }
    for (k, v) in cfg_map {
        let k = format!("CARGO_CFG_{}", super::envify(&k));
        match v {
            Some(list) => { cmd.env(&k, list.join(",")); }
            None => { cmd.env(&k, ""); }
        }
    }

    // Gather the set of native dependencies that this package has along with
    // some other variables to close over.
    //
    // This information will be used at build-time later on to figure out which
    // sorts of variables need to be discovered at that time.
    let lib_deps = {
        dependencies.iter().filter_map(|unit| {
            if unit.profile.run_custom_build {
                Some((unit.pkg.manifest().links().unwrap().to_string(),
                      unit.pkg.package_id().clone()))
            } else {
                None
            }
        }).collect::<Vec<_>>()
    };
    let pkg_name = unit.pkg.to_string();
    let build_state = cx.build_state.clone();
    let id = unit.pkg.package_id().clone();
    let output_file = build_output.parent().unwrap().join("output");
    let all = (id.clone(), pkg_name.clone(), build_state.clone(),
               output_file.clone());
    let build_scripts = super::load_build_deps(cx, unit);
    let kind = unit.kind;
    let json_messages = cx.build_config.json_messages;

    // Check to see if the build script as already run, and if it has keep
    // track of whether it has told us about some explicit dependencies
    let prev_output = BuildOutput::parse_file(&output_file, &pkg_name).ok();
    let rerun_if_changed = match prev_output {
        Some(ref prev) => prev.rerun_if_changed.clone(),
        None => Vec::new(),
    };
    cx.build_explicit_deps.insert(*unit, (output_file.clone(), rerun_if_changed));

    fs::create_dir_all(&script_output)?;
    fs::create_dir_all(&build_output)?;

    // Prepare the unit of "dirty work" which will actually run the custom build
    // command.
    //
    // Note that this has to do some extra work just before running the command
    // to determine extra environment variables and such.
    let dirty = Work::new(move |state| {
        // Make sure that OUT_DIR exists.
        //
        // If we have an old build directory, then just move it into place,
        // otherwise create it!
        if fs::metadata(&build_output).is_err() {
            fs::create_dir(&build_output).chain_error(|| {
                internal("failed to create script output directory for \
                          build command")
            })?;
        }

        // For all our native lib dependencies, pick up their metadata to pass
        // along to this custom build command. We're also careful to augment our
        // dynamic library search path in case the build script depended on any
        // native dynamic libraries.
        {
            let build_state = build_state.outputs.lock().unwrap();
            for (name, id) in lib_deps {
                let key = (id.clone(), kind);
                let state = build_state.get(&key).chain_error(|| {
                    internal(format!("failed to locate build state for env \
                                      vars: {}/{:?}", id, kind))
                })?;
                let data = &state.metadata;
                for &(ref key, ref value) in data.iter() {
                    cmd.env(&format!("DEP_{}_{}", super::envify(&name),
                                     super::envify(key)), value);
                }
            }
            if let Some(build_scripts) = build_scripts {
                super::add_plugin_deps(&mut cmd, &build_state,
                                            &build_scripts)?;
            }
        }

        // And now finally, run the build command itself!
        state.running(&cmd);
        let output = cmd.exec_with_streaming(
            &mut |out_line| { state.stdout(out_line); Ok(()) },
            &mut |err_line| { state.stderr(err_line); Ok(()) },
        ).map_err(|mut e| {
            e.desc = format!("failed to run custom build command for `{}`\n{}",
                             pkg_name, e.desc);
            Human(e)
        })?;
        paths::write(&output_file, &output.stdout)?;

        // After the build command has finished running, we need to be sure to
        // remember all of its output so we can later discover precisely what it
        // was, even if we don't run the build command again (due to freshness).
        //
        // This is also the location where we provide feedback into the build
        // state informing what variables were discovered via our script as
        // well.
        let parsed_output = BuildOutput::parse(&output.stdout, &pkg_name)?;

        if json_messages {
            let library_paths = parsed_output.library_paths.iter().map(|l| {
                l.display().to_string()
            }).collect::<Vec<_>>();
            machine_message::emit(machine_message::BuildScript {
                package_id: &id,
                linked_libs: &parsed_output.library_links,
                linked_paths: &library_paths,
                cfgs: &parsed_output.cfgs,
            });
        }

        build_state.insert(id, kind, parsed_output);
        Ok(())
    });

    // Now that we've prepared our work-to-do, we need to prepare the fresh work
    // itself to run when we actually end up just discarding what we calculated
    // above.
    let fresh = Work::new(move |_tx| {
        let (id, pkg_name, build_state, output_file) = all;
        let output = match prev_output {
            Some(output) => output,
            None => BuildOutput::parse_file(&output_file, &pkg_name)?,
        };
        build_state.insert(id, kind, output);
        Ok(())
    });

    Ok((dirty, fresh))
}

impl BuildState {
    pub fn new(config: &super::BuildConfig) -> BuildState {
        let mut overrides = HashMap::new();
        let i1 = config.host.overrides.iter().map(|p| (p, Kind::Host));
        let i2 = config.target.overrides.iter().map(|p| (p, Kind::Target));
        for ((name, output), kind) in i1.chain(i2) {
            overrides.insert((name.clone(), kind), output.clone());
        }
        BuildState {
            outputs: Mutex::new(HashMap::new()),
            overrides: overrides,
        }
    }

    fn insert(&self, id: PackageId, kind: Kind, output: BuildOutput) {
        self.outputs.lock().unwrap().insert((id, kind), output);
    }

    fn has_override(&self, unit: &Unit) -> bool {
        let key = unit.pkg.manifest().links().map(|l| (l.to_string(), unit.kind));
        match key.and_then(|k| self.overrides.get(&k)) {
            Some(output) => {
                self.insert(unit.pkg.package_id().clone(), unit.kind,
                            output.clone());
                true
            }
            None => false,
        }
    }
}

impl BuildOutput {
    pub fn parse_file(path: &Path, pkg_name: &str) -> CargoResult<BuildOutput> {
        let contents = paths::read_bytes(path)?;
        BuildOutput::parse(&contents, pkg_name)
    }

    // Parses the output of a script.
    // The `pkg_name` is used for error messages.
    pub fn parse(input: &[u8], pkg_name: &str) -> CargoResult<BuildOutput> {
        let mut library_paths = Vec::new();
        let mut library_links = Vec::new();
        let mut cfgs = Vec::new();
        let mut metadata = Vec::new();
        let mut rerun_if_changed = Vec::new();
        let mut warnings = Vec::new();
        let whence = format!("build script of `{}`", pkg_name);

        for line in input.split(|b| *b == b'\n') {
            let line = match str::from_utf8(line) {
                Ok(line) => line.trim(),
                Err(..) => continue,
            };
            let mut iter = line.splitn(2, ':');
            if iter.next() != Some("cargo") {
                // skip this line since it doesn't start with "cargo:"
                continue;
            }
            let data = match iter.next() {
                Some(val) => val,
                None => continue
            };

            // getting the `key=value` part of the line
            let mut iter = data.splitn(2, '=');
            let key = iter.next();
            let value = iter.next();
            let (key, value) = match (key, value) {
                (Some(a), Some(b)) => (a, b.trim_right()),
                // line started with `cargo:` but didn't match `key=value`
                _ => bail!("Wrong output in {}: `{}`", whence, line),
            };

            match key {
                "rustc-flags" => {
                    let (libs, links) =
                        BuildOutput::parse_rustc_flags(value, &whence)?;
                    library_links.extend(links.into_iter());
                    library_paths.extend(libs.into_iter());
                }
                "rustc-link-lib" => library_links.push(value.to_string()),
                "rustc-link-search" => library_paths.push(PathBuf::from(value)),
                "rustc-cfg" => cfgs.push(value.to_string()),
                "warning" => warnings.push(value.to_string()),
                "rerun-if-changed" => rerun_if_changed.push(value.to_string()),
                _ => metadata.push((key.to_string(), value.to_string())),
            }
        }

        Ok(BuildOutput {
            library_paths: library_paths,
            library_links: library_links,
            cfgs: cfgs,
            metadata: metadata,
            rerun_if_changed: rerun_if_changed,
            warnings: warnings,
        })
    }

    pub fn parse_rustc_flags(value: &str, whence: &str)
                             -> CargoResult<(Vec<PathBuf>, Vec<String>)> {
        let value = value.trim();
        let mut flags_iter = value.split(|c: char| c.is_whitespace())
                                  .filter(|w| w.chars().any(|c| !c.is_whitespace()));
        let (mut library_links, mut library_paths) = (Vec::new(), Vec::new());
        loop {
            let flag = match flags_iter.next() {
                Some(f) => f,
                None => break
            };
            if flag != "-l" && flag != "-L" {
                bail!("Only `-l` and `-L` flags are allowed in {}: `{}`",
                      whence, value)
            }
            let value = match flags_iter.next() {
                Some(v) => v,
                None => bail!("Flag in rustc-flags has no value in {}: `{}`",
                              whence, value)
            };
            match flag {
                "-l" => library_links.push(value.to_string()),
                "-L" => library_paths.push(PathBuf::from(value)),

                // was already checked above
                _ => bail!("only -l and -L flags are allowed")
            };
        }
        Ok((library_paths, library_links))
    }
}

/// Compute the `build_scripts` map in the `Context` which tracks what build
/// scripts each package depends on.
///
/// The global `build_scripts` map lists for all (package, kind) tuples what set
/// of packages' build script outputs must be considered. For example this lists
/// all dependencies' `-L` flags which need to be propagated transitively.
///
/// The given set of targets to this function is the initial set of
/// targets/profiles which are being built.
pub fn build_map<'b, 'cfg>(cx: &mut Context<'b, 'cfg>,
                           units: &[Unit<'b>])
                           -> CargoResult<()> {
    let mut ret = HashMap::new();
    for unit in units {
        build(&mut ret, cx, unit)?;
    }
    cx.build_scripts.extend(ret.into_iter().map(|(k, v)| {
        (k, Arc::new(v))
    }));
    return Ok(());

    // Recursive function to build up the map we're constructing. This function
    // memoizes all of its return values as it goes along.
    fn build<'a, 'b, 'cfg>(out: &'a mut HashMap<Unit<'b>, BuildScripts>,
                           cx: &Context<'b, 'cfg>,
                           unit: &Unit<'b>)
                           -> CargoResult<&'a BuildScripts> {
        // Do a quick pre-flight check to see if we've already calculated the
        // set of dependencies.
        if out.contains_key(unit) {
            return Ok(&out[unit])
        }

        let mut ret = BuildScripts::default();

        if !unit.target.is_custom_build() && unit.pkg.has_custom_build() {
            add_to_link(&mut ret, unit.pkg.package_id(), unit.kind);
        }
        for unit in cx.dep_targets(unit)?.iter() {
            let dep_scripts = build(out, cx, unit)?;

            if unit.target.for_host() {
                ret.plugins.extend(dep_scripts.to_link.iter()
                                              .map(|p| &p.0).cloned());
            } else if unit.target.linkable() {
                for &(ref pkg, kind) in dep_scripts.to_link.iter() {
                    add_to_link(&mut ret, pkg, kind);
                }
            }
        }

        let prev = out.entry(*unit).or_insert(BuildScripts::default());
        for (pkg, kind) in ret.to_link {
            add_to_link(prev, &pkg, kind);
        }
        prev.plugins.extend(ret.plugins);
        Ok(prev)
    }

    // When adding an entry to 'to_link' we only actually push it on if the
    // script hasn't seen it yet (e.g. we don't push on duplicates).
    fn add_to_link(scripts: &mut BuildScripts, pkg: &PackageId, kind: Kind) {
        if scripts.seen_to_link.insert((pkg.clone(), kind)) {
            scripts.to_link.push((pkg.clone(), kind));
        }
    }
}
