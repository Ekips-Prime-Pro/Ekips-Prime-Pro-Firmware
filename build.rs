//! Makes `memory.x` visible to the linker (cortex-m-rt's `link.x` includes it).

use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));
    fs::copy("memory.x", out.join("memory.x")).expect("memory.x must exist");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
