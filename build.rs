//! Embeds the icon and version information into the executable.
//!
//! No custom manifest: MinGW always links its own default one, and two
//! manifests would leave Windows to pick either. Runs windres directly because
//! the embed-resource crate passes OUT_DIR unquoted, which breaks on paths with spaces.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=resources");
    let object = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("resources.o");
    let version = |part| format!("-DVER_{part}={}", std::env::var(format!("CARGO_PKG_VERSION_{part}")).unwrap());
    let status = Command::new("windres")
        .current_dir("resources")
        .args([version("MAJOR"), version("MINOR"), version("PATCH")])
        .args(["-i", "app.rc", "-o"])
        .arg(&object)
        .status()
        .expect("windres (MinGW) must be on PATH");
    assert!(status.success(), "windres failed");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
