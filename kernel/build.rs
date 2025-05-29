//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory (wherever `Cargo.toml` is). However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! a rebuild of the application with new memory settings is ensured after updating `memory.x`.

use mseq_core::Note;
use mseq_tracks::index::load_from_file;
use postcard::to_stdvec;
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

    //Load an acid track
    // let tracks = load_from_file("../res/index.toml").unwrap();
    // let bytes = to_stdvec(&tracks[0]).unwrap();
    // let mut bin_file = File::create("../res/test.bin").unwrap();
    // bin_file.write_all(&bytes).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
