pub use self::imp::*;

#[cfg(all(
    regex_runtime_teddy_ssse3,
    target_arch = "x86_64",
))]
mod imp;

#[cfg(not(all(
    regex_runtime_teddy_ssse3,
    target_arch = "x86_64",
)))]
#[path = "fallback.rs"]
mod imp;
