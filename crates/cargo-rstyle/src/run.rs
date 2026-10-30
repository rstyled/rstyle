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
    let driver = exe.with_file_name("rstyle-driver");
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
        if diagnostic_environment(&key, &value) {
            continue;
        }
        if control_environment(&key) {
            command.env_remove(key);
        }
    }
}

fn control_environment(key: &OsStr) -> bool {
    key.to_str().is_some_and(|key| {
        key.starts_with("RSTYLE_DIAGNOSTIC_")
            || matches!(
                key,
                "RSTYLE_CAPTURE_PLAN"
                    | "RSTYLE_COMPILER_ADVISORY"
                    | "RSTYLE_COMPILER_ADVISORY_ONLY"
                    | "RSTYLE_STYLE_INPUTS_ONLY"
                    | "RSTYLE_COMPILER_PATTERNS"
                    | "RSTYLE_COMPILER_SOURCE_ROOT"
                    | "RSTYLE_DRIVER"
                    | "RSTYLE_HIR_DOCUMENT_DIR"
                    | "RSTYLE_HIR_FACTS_FILE"
                    | "RSTYLE_HIR_EXTRACTION_PROFILE"
                    | "RSTYLE_LOCAL_MUTATION_FACTS"
                    | "RSTYLE_LOCAL_SOURCE_FACTS"
                    | "RSTYLE_OUT_DIR"
                    | "RSTYLE_PDG"
                    | "RSTYLE_PERF"
                    | "RSTYLE_ROOT"
            )
    })
}

fn diagnostic_environment(key: &OsStr, value: &OsStr) -> bool {
    key == "RSTYLE_DIAGNOSTIC_FACTS_DIR"
        || (key == "RSTYLE_PERF"
            || key == "RSTYLE_DIAGNOSTIC_CAPTURE_ONLY"
            || key == "RSTYLE_DIAGNOSTIC_GENERATED_SOURCES")
            && value == "1"
        || (key == "RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS"
            || key == "RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS"
            || key == "RSTYLE_DIAGNOSTIC_SHARED_SOURCES" && value == "1")
            && std::env::var_os("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY").as_deref()
                == Some(OsStr::new("1"))
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
            "RSTYLE_HIR_FACTS_FILE",
            "RSTYLE_COMPILER_ADVISORY_ONLY",
            "RSTYLE_STYLE_INPUTS_ONLY",
            "RSTYLE_DIAGNOSTIC_CAPTURE_ONLY",
        ] {
            assert!(control_environment(OsStr::new(name)), "{name}");
        }
    }

    #[test]
    fn toolchain_is_pinned_nightly() {
        assert!(pinned_toolchain().starts_with("nightly-"));
    }
}
