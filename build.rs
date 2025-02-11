//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Tell Cargo to link the Pico SDK
    // println!(
    //     "cargo:rustc-link-search={}",
    //     env::var("PICO_SDK_PATH").unwrap()
    // );

    // Link the C standard library
    // println!("cargo::rustc-link-lib=static=c");

    // Build the Pico SDK C code
    // cc::Build::new()
    //     .file("src/jartis.c") // Path to your C wrapper file
    //     .include(&env::var("PICO_SDK_PATH").unwrap())
    //     .compile("jartis");

    // Generate Rust bindings for the C wrapper
    // let bindings = bindgen::Builder::default()
    //     .header("jartis.h") // Path to your C header file
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     .generate()
    //     .expect("Unable to generate bindings");
    //
    // // Write the bindings to a file
    // bindings
    //     .write_to_file(out.join("bindings.rs"))
    //     .expect("Couldn't write bindings!");

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Build C library
    // Command::new("./c_build.sh").output().unwrap();

    // println!("cargo::rustc-link-lib=static=jartis");
    // println!("cargo::rustc-link-search=native=target/thumbv6m-none-eabi/debug/deps/libc.a");
}
