//! Populates the correct memory file depending on the passed in feature
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// features are transformed into environment variables:
// https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
const FEATURE_FEATHER_M0: &str = "CARGO_FEATURE_FEATHER_M0";
const FEATURE_FEATHER_M4: &str = "CARGO_FEATURE_FEATHER_M4";

#[derive(Debug)]
enum Target {
    M0,
    M4,
}

fn main() {
    // check which target we're trying to build for
    let target = {
        if env::var_os(FEATURE_FEATHER_M0).is_some() {
            Target::M0
        } else if env::var_os(FEATURE_FEATHER_M4).is_some() {
            Target::M4
        } else {
            panic!("invalid feature set");
        }
    };

    // check the target triple as such
    //   I'd like to dynamically just set the build target, but I tried and
    //   it doesn't seem to work, and all of the other crates will have been
    //   built before this executes so it kinda needs to happen before this
    //   script anyways...
    match target {
        Target::M0 => {
            if env::var_os("TARGET").unwrap() != "thumbv6m-none-eabi" {
                panic!("incorrect target triple for target {:?}", target);
            }
        }
        Target::M4 => {
            if env::var_os("TARGET").unwrap() != "thumbv7em-none-eabihf" {
                panic!("incorrect target triple for target {:?}", target);
            }
        }
    }

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut mem_file = File::create(out.join("memory.x")).unwrap();

    match target {
        Target::M0 => mem_file.write_all(include_bytes!("m0-memory.x")).unwrap(),
        Target::M4 => mem_file.write_all(include_bytes!("m4-memory.x")).unwrap(),
    }
    println!("cargo:rustc-link-search={}", out.display());

    // Specify which memory files are critical and require recompilation
    println!("cargo:rerun-if-changed=m0-memory.x");
    println!("cargo:rerun-if-changed=m4-memory.x");
}
