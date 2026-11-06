//! Compiler tool selection and isolation of inherited extraction controls.

use serde::Deserialize;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

const TOOLCHAIN_FILE: &str = include_str!("../../../rust-toolchain.toml");

#[derive(Deserialize)]
struct ToolchainFile {
    toolchain: ToolchainSection,
}

#[derive(Deserialize)]
struct ToolchainSection {
    channel: String,
}

/// The nightly the driver was built against; the whole `cargo check` must use it
/// or dependency metadata would come from an incompatible rustc (E0514).
pub fn pinned_toolchain() -> String {
    let file: ToolchainFile = toml::from_str(TOOLCHAIN_FILE).expect("valid rust-toolchain.toml");
    file.toolchain.channel
}

/// The driver binary next to this executable (or `$RSTYLE_DRIVER`).
// rstyle-allow(side_effect_density): IO boundary: locates the driver binary on disk
pub fn driver_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("RSTYLE_DRIVER") {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let driver = exe.with_file_name(format!("rstyle-driver{}", std::env::consts::EXE_SUFFIX));
    driver.exists().then_some(driver.clone()).ok_or_else(|| {
        format!(
            "rstyle-driver not found at {} (build it with `cargo build -p rstyle-driver`, \
                 or install both binaries side by side)",
            driver.display()
        )
    })
}

/// Commands select their own extraction mode; inherited control settings cannot
/// change it. Preserve project inputs, including Cargo's build-script rustc-env
/// metadata. Resolve RSTYLE_DRIVER before calling this helper.
pub fn clear_rstyle_environment(command: &mut Command) {
    for (key, value) in std::env::vars_os() {
        if key == "RSTYLE_PERF" && value == "1" {
            continue;
        }
        if control_environment(&key) {
            command.env_remove(key);
        }
    }
}

fn control_environment(key: &OsStr) -> bool {
    key.to_str().is_some_and(|key| {
        matches!(
            key,
            "RSTYLE_CAPTURE_PLAN"
                | "RSTYLE_COMPILER_SOURCE_ROOT"
                | "RSTYLE_CPG_DIR"
                | "RSTYLE_CPG_ROOT"
                | "RSTYLE_CPG_COMPRESSION"
                | "RSTYLE_DOCUMENT_DIR"
                | "RSTYLE_DRIVER"
                | "RSTYLE_PERF"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compilation_inputs_are_distinct_from_extraction_controls() {
        for name in [
            "RSTYLE_PRODUCER_REVISION",
            "RSTYLE_RUSTC_VERSION",
            "RSTYLE_RUSTC_COMMIT",
            "RSTYLE_PROJECT_VALUE",
            "OUT_DIR",
        ] {
            assert!(!control_environment(OsStr::new(name)), "{name}");
        }
        for name in [
            "RSTYLE_CAPTURE_PLAN",
            "RSTYLE_DOCUMENT_DIR",
            "RSTYLE_CPG_DIR",
            "RSTYLE_PERF",
        ] {
            assert!(control_environment(OsStr::new(name)), "{name}");
        }
    }

    #[test]
    fn toolchain_is_pinned_nightly() {
        assert!(pinned_toolchain().starts_with("nightly-"));
    }
}
