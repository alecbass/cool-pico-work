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

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Build Jartis C library
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let lib_dir = manifest_dir.join("build");

    if !lib_dir.exists() {
        panic!(
            "Library directory build/ does not exist relative to project root ({})",
            manifest_dir.display()
        );
    }

    let lib_path_str = lib_dir.to_str().expect("Library path is not valid UTF-8");
    println!("cargo:rustc-link-search=native={}", lib_path_str);
    println!("cargo:rustc-link-lib=static=jartis");

    let lib_file_path = lib_dir.join("libjartis.a");

    if !lib_file_path.exists() {
        panic!(
            "Library file libjartis.a does not exist in build/ directory ({})",
            lib_file_path.display()
        );
    }
    println!("cargo:rerun-if-changed={}", lib_file_path.display());

    // Add rerun-if-changed for the build directory itself, in case the file is replaced
    println!("cargo:rerun-if-changed={}", lib_dir.display());

    let arm_embedded_dir = env!("GCC_ARM_EMBEDDED_TOOLCHAIN");
    let c_file_path = PathBuf::from(format!(
        "{arm_embedded_dir}/arm-none-eabi/lib/thumb/v6-m/nofp"
    ));

    if !c_file_path.exists() {
        panic!(
            "C library file libc.a does not exist in nix store directory ({})",
            c_file_path.display()
        );
    }

    // This tells the -lc flag in .cargo/config.toml where the C library is
    println!("cargo:rustc-link-search=native={}", c_file_path.display());
    // println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rerun-if-changed={}", c_file_path.display());

    // TODO: Get proper Jartis and Pico C SDK bindings
    // let bindings = bindgen::Builder::default()
    //     // The input header we would like to generate
    //     // bindings for.
    //     .header("../jartis/src/jartis.h")
    //     // .header_contents("jartis.h", jartis_h_contents)
    //     // Tell cargo to invalidate the built crate whenever any of the
    //     // included header files changed.
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     // Finish the builder and generate the bindings.
    //     .generate()
    //     // Unwrap the Result and panic on failure.
    //     .expect("Unable to generate bindings");
    //
    // // Write the bindings to the $OUT_DIR/bindings.rs file.
    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // bindings
    //     .write_to_file(out_path.join("bindings.rs"))
    //     .expect("Couldn't write bindings!");
}
