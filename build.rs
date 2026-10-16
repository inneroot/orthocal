//! Exposes `GIT_HASH` (short commit hash) as a compile-time env var so the
//! binary version is `CARGO_PKG_VERSION-<hash>` with a single source of
//! truth (Cargo.toml). Falls back to no suffix when not built from a git
//! checkout or when git is unavailable.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let hash = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    match hash {
        Some(h) => println!("cargo:rustc-env=GIT_HASH={h}"),
        None => println!("cargo:warning=GIT_HASH unset: not in a git checkout"),
    }
}
