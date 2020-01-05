use std::process::Command;
use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed={}", "DEB_HOST_GNU_TYPE");
    let exe = env::var("DEB_HOST_GNU_TYPE")
        .map(|t| t.to_string() + "-gcc")
        .unwrap_or_else(|_| String::from("gcc"));
    let cmd = format!("{} -print-search-dirs | sed -ne 's/^install: //p'", exe).to_string();
    let search_dir = Command::new("sh")
        .args(&["-c", &cmd])
        .output().expect("failed to find gcc install dir").stdout;
    println!("cargo:rustc-link-lib=static=backtrace");
    println!("cargo:rustc-link-search=native={}", String::from_utf8(search_dir).unwrap().trim_end());
    println!("dh-cargo:deb-built-using=backtrace=0~={}", "libgcc-[0-9]+-dev .*");
}
