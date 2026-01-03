//! Build script for Tarqeem compiler
//!
//! Sets TARQEEM_RUNTIME_PATH to point to the built runtime library,
//! enabling automatic discovery during compilation.

fn main() {
    // Re-run if runtime library changes
    println!("cargo:rerun-if-changed=runtime-rs/src");

    // Calculate runtime library path based on build profile
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let lib_name = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };
    let runtime_path = std::path::PathBuf::from(&manifest_dir)
        .join("target")
        .join(&profile)
        .join(lib_name);

    // Set environment variable for the compiler
    println!(
        "cargo:rustc-env=TARQEEM_RUNTIME_PATH={}",
        runtime_path.display()
    );
}
