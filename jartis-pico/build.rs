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
    //     .header("test.h") // Path to your C header file
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     .generate()
    //     .expect("Unable to generate bindings");

    // Write the bindings to a file
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

    // 1. Get the project root directory
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");

    // 2. Construct the path to the directory containing the library
    //    Assuming 'build/' is directly inside your project root.
    let lib_dir = manifest_dir.join("build");

    // Check if the directory exists (optional but good for diagnostics)
    if !lib_dir.exists() {
        panic!(
            "Library directory build/ does not exist relative to project root ({})",
            manifest_dir.display()
        );
    }
    let lib_path_str = lib_dir.to_str().expect("Library path is not valid UTF-8");

    // 3. Tell rustc where to find the library
    //    'native=' specifies a directory for native libraries.
    println!("cargo:rustc-link-search=native={}", lib_path_str);

    // 4. Tell rustc to link the static library
    //    'static=' links a static library. The name 'jartis' is derived
    //    from 'libjartis.a' by removing the 'lib' prefix and '.a' suffix.
    println!("cargo:rustc-link-lib=static=jartis");

    // 5. (Optional but Recommended) Re-run build script if the library changes
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

    let c_file_path = PathBuf::from(
        "/nix/store/961aqd7g5k8k4zfsvp8yyj7r3bczd4c6-gcc-arm-embedded-13.3.rel1/arm-none-eabi/lib/thumb/v6-m/nofp",
    );

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
}
