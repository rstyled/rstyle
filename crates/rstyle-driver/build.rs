//! The driver links `librustc_driver` dynamically. Bake the sysroot lib dir into
//! the binary's rpath so it runs without `DYLD_*` / `LD_LIBRARY_PATH` tweaks
//! (macOS SIP strips `DYLD_*` variables in many spawn paths).

use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let output = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("failed to run rustc --print sysroot");
    let sysroot = String::from_utf8(output.stdout).expect("sysroot is not utf-8");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", sysroot.trim());
    println!("cargo:rerun-if-changed=build.rs");
    compiler_identity();
}

fn compiler_identity() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let output = Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("rustc identity");
    assert!(output.status.success(), "rustc identity failed");
    let identity = String::from_utf8(output.stdout).expect("rustc identity UTF-8");
    let version = identity.lines().next().expect("rustc version");
    let commit = identity
        .lines()
        .find_map(|line| line.strip_prefix("commit-hash: "))
        .expect("rustc commit");

    println!("cargo:rustc-env=RSTYLE_RUSTC_VERSION={version}");
    println!("cargo:rustc-env=RSTYLE_RUSTC_COMMIT={commit}");
    producer_revision();
}

fn producer_revision() {
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("producer revision");
    assert!(revision.status.success(), "missing producer revision");
    let revision = String::from_utf8(revision.stdout).expect("revision UTF-8");
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .expect("producer worktree status");
    let dirty = if status.stdout.is_empty() {
        ""
    } else {
        "+dirty"
    };

    println!(
        "cargo:rustc-env=RSTYLE_PRODUCER_REVISION={}{dirty}",
        revision.trim()
    );
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
}
