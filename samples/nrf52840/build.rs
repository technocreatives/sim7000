use std::{fs::File, io::Write, path::PathBuf};

const MEMORY_X: &[u8] = include_bytes!("memory.x");

fn main() {
    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("rustc did not set an output directory"));
    let memory_x_path = out_dir.join("memory.x");

    File::create(memory_x_path)
        .expect("Could not create/open memory file")
        .write_all(MEMORY_X)
        .expect("Failed to write to contents of memory.x file");

    println!("cargo:rustc-link-search={}", out_dir.to_string_lossy());

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
}
