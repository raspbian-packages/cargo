use std::env;

use cargo::ops;
use cargo::util::{CliResult, CliError, Config};
use cargo::util::important_paths::{find_root_manifest_for_cwd};

#[derive(RustcDecodable)]
struct Options {
    flag_package: Vec<String>,
    flag_target: Option<String>,
    flag_manifest_path: Option<String>,
    flag_verbose: bool,
    flag_quiet: bool,
    flag_color: Option<String>,
}

pub const USAGE: &'static str = "
Remove artifacts that cargo has generated in the past

Usage:
    cargo clean [options]

Options:
    -h, --help                   Print this message
    -p SPEC, --package SPEC ...  Package to clean artifacts for
    --manifest-path PATH         Path to the manifest to the package to clean
    --target TRIPLE              Target triple to clean output for (default all)
    -v, --verbose                Use verbose output
    -q, --quiet                  No output printed to stdout
    --color WHEN                 Coloring: auto, always, never

If the --package argument is given, then SPEC is a package id specification
which indicates which package's artifacts should be cleaned out. If it is not
given, then all packages' artifacts are removed. For more information on SPEC
and its format, see the `cargo help pkgid` command.
";

pub fn execute(options: Options, config: &Config) -> CliResult<Option<()>> {
    try!(config.shell().set_verbosity(options.flag_verbose, options.flag_quiet));
    try!(config.shell().set_color_config(options.flag_color.as_ref().map(|s| &s[..])));
    debug!("executing; cmd=cargo-clean; args={:?}", env::args().collect::<Vec<_>>());

    let root = try!(find_root_manifest_for_cwd(options.flag_manifest_path));
    let opts = ops::CleanOptions {
        config: config,
        spec: &options.flag_package,
        target: options.flag_target.as_ref().map(|s| &s[..]),
    };
    ops::clean(&root, &opts).map(|_| None).map_err(|err| {
      CliError::from_boxed(err, 101)
    })
}
