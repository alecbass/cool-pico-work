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

    let does_embassy_memory_mapping_exist = match std::fs::exists(out.join("link-rp.x")) {
        Ok(result) => result,
        _ => false,
    };

    if let Ok(dir) = std::fs::read_dir(out) {
        println!("DIR: {:?}", dir);

        for item in dir {
            println!("ITEM: {:?}", item);
        }
    }
    // if does_embassy_memory_mapping_exist {
    std::fs::remove_file(out.join("link-rp.x")).unwrap();
    // }

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // let arm_embedded_dir = env!("GCC_ARM_EMBEDDED_TOOLCHAIN");
    // let c_file_path = PathBuf::from(
    //     format!("{arm_embedded_dir}/arm-none-eabi/lib/thumb/v6-m/nofp")
    // );
    //
    // if !c_file_path.exists() {
    //     panic!(
    //         "C library file libc.a does not exist in nix store directory ({})",
    //         c_file_path.display()
    //     );
    // }
    //
    // // This tells the -lc flag in .cargo/config.toml where the C library is
    // println!("cargo:rustc-link-search=native={}", c_file_path.display());
    // // println!("cargo:rustc-link-lib=static=c");
    // println!("cargo:rerun-if-changed={}", c_file_path.display());
}
