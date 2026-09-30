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
}
