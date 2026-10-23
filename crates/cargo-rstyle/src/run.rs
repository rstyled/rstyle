//! Runs `cargo check` with `rstyle-driver` wrapped around rustc for workspace crates.

use serde::Deserialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
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

/// Cargo does not hash the wrapper's contents, so a rebuilt driver would keep
/// serving stale facts. Feeding a driver fingerprint into RUSTFLAGS changes
/// cargo's own fingerprint exactly when the driver changes. The semantic (PDG) mode
/// adds a suffix, so style and `patterns` runs keep separate cargo fingerprints and
/// never serve each other's cached (PDG-less or PDG-bearing) facts.
// rstyle-allow(side_effect_density): IO boundary: stats the driver binary
fn driver_fingerprint(driver: &Path, pdg: bool) -> String {
    let meta = std::fs::metadata(driver).ok();
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let len = meta.map_or(0, |m| m.len());
    format!("rstyle{modified}x{len}{}", if pdg { "p" } else { "" })
}

pub struct Paths<'a> {
    pub driver: &'a Path,
    pub root: &'a Path,
    pub target_dir: &'a Path,
    pub out_dir: &'a Path,
    /// Ask the driver for dependence graphs (`patterns`); style runs leave this off.
    pub pdg: bool,
}

pub struct Selection<'a> {
    pub packages: &'a [String],
    pub all_targets: bool,
    pub manifest: Option<&'a Path>,
}

fn cargo_args(selection: &Selection<'_>) -> Vec<String> {
    let scope: Vec<String> = if selection.packages.is_empty() {
        vec!["--workspace".to_string()]
    } else {
        selection
            .packages
            .iter()
            .flat_map(|package| ["-p".to_string(), package.clone()])
            .collect()
    };
    let targets = selection.all_targets.then(|| "--all-targets".to_string());
    ["check", "--quiet"]
        .map(String::from)
        .into_iter()
        .chain(scope)
        .chain(targets)
        .collect()
}

fn command(paths: &Paths<'_>, selection: &Selection<'_>) -> Command {
    let existing = std::env::var("RUSTFLAGS").unwrap_or_default();
    let flags = format!(
        "{existing} -Cmetadata={}",
        driver_fingerprint(paths.driver, paths.pdg)
    );
    let manifest = selection
        .manifest
        .into_iter()
        .flat_map(|path| [OsStr::new("--manifest-path"), path.as_os_str()]);
    let toolchain = pinned_toolchain();
    let envs: [(&str, &OsStr); 6] = [
        ("RUSTUP_TOOLCHAIN", OsStr::new(&toolchain)),
        ("RUSTC_WORKSPACE_WRAPPER", paths.driver.as_os_str()),
        ("RUSTC_WRAPPER", OsStr::new("")),
        ("RUSTFLAGS", OsStr::new(flags.trim())),
        ("CARGO_TARGET_DIR", paths.target_dir.as_os_str()),
        ("RSTYLE_OUT_DIR", paths.out_dir.as_os_str()),
    ];

    // Only `patterns` asks for dependence graphs; a stale value must never leak into style runs.
    let pdg = paths.pdg.then_some(("RSTYLE_PDG", OsStr::new("1")));
    let mut cmd = Command::new("cargo");
    clear_rstyle_environment(&mut cmd);
    cmd.args(cargo_args(selection))
        .args(manifest)
        .envs(envs)
        .env("RSTYLE_ROOT", paths.root)
        .env_remove("RUSTC")
        .env_remove("RSTYLE_HIR_DOCUMENT_DIR")
        .env_remove("RSTYLE_HIR_FACTS_FILE")
        .env_remove("RSTYLE_PDG")
        .envs(pdg);
    cmd
}

/// Commands select their own extraction mode; inherited producer settings cannot
/// change it. Resolve RSTYLE_DRIVER before calling this helper.
pub fn clear_rstyle_environment(command: &mut Command) {
    for (key, value) in std::env::vars_os() {
        if diagnostic_environment(&key, &value) {
            continue;
        }
        if key.to_string_lossy().starts_with("RSTYLE_") {
            command.env_remove(key);
        }
    }
}

fn diagnostic_environment(key: &OsStr, value: &OsStr) -> bool {
    key == "RSTYLE_DIAGNOSTIC_FACTS_DIR"
        || (key == "RSTYLE_PERF" || key == "RSTYLE_DIAGNOSTIC_CAPTURE_ONLY") && value == "1"
        || (key == "RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS"
            || key == "RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS"
            || key == "RSTYLE_DIAGNOSTIC_SHARED_SOURCES" && value == "1")
            && std::env::var_os("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY").as_deref()
                == Some(OsStr::new("1"))
}

/// Compiles the selected crates so the driver writes its facts files.
// rstyle-allow(side_effect_density): IO boundary: spawns cargo
pub fn cargo_check(paths: &Paths<'_>, selection: &Selection<'_>) -> Result<(), String> {
    let output = command(paths, selection)
        .output()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolchain_is_pinned_nightly() {
        assert!(pinned_toolchain().starts_with("nightly-"));
    }

    #[test]
    fn legacy_command_explicitly_removes_hir_destinations() {
        let paths = Paths {
            driver: Path::new("driver"),
            root: Path::new("root"),
            target_dir: Path::new("build"),
            out_dir: Path::new("facts"),
            pdg: false,
        };
        let selection = Selection {
            packages: &[],
            all_targets: false,
            manifest: None,
        };
        let command = command(&paths, &selection);
        for key in ["RSTYLE_HIR_DOCUMENT_DIR", "RSTYLE_HIR_FACTS_FILE"] {
            assert!(
                command
                    .get_envs()
                    .any(|(name, value)| name == key && value.is_none())
            );
        }
    }

    #[test]
    fn default_selection_is_whole_workspace() {
        let selection = Selection {
            packages: &[],
            all_targets: false,
            manifest: None,
        };
        assert_eq!(cargo_args(&selection), ["check", "--quiet", "--workspace"]);
    }

    #[test]
    fn package_selection_and_all_targets() {
        let packages = vec!["a".to_string(), "b".to_string()];
        let selection = Selection {
            packages: &packages,
            all_targets: true,
            manifest: None,
        };
        assert_eq!(
            cargo_args(&selection),
            ["check", "--quiet", "-p", "a", "-p", "b", "--all-targets"]
        );
    }
}
