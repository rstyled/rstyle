//! `cargo metadata` view of the workspace: where it is, and which crates to expect facts for.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub workspace_root: PathBuf,
    pub target_directory: PathBuf,
    pub packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: String,
}

pub fn load(manifest: Option<&Path>) -> Result<Metadata, String> {
    load_with_toolchain(manifest, None)
}

pub fn load_with_toolchain(
    manifest: Option<&Path>,
    toolchain: Option<&str>,
) -> Result<Metadata, String> {
    let mut cmd = Command::new("cargo");

    cmd.args(["metadata", "--format-version", "1", "--no-deps"])
        .envs(toolchain.map(|channel| ("RUSTUP_TOOLCHAIN", channel)));
    if let Some(path) = manifest {
        cmd.arg("--manifest-path").arg(path);
    }
    let output = cmd.output().map_err(|e| format!("cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))
}

impl Metadata {
    /// Crate names (as rustc sees them) of the selected packages; all when none selected.
    pub fn crate_names(&self, selected: &[String]) -> Vec<String> {
        self.packages
            .iter()
            .filter(|p| selected.is_empty() || selected.contains(&p.name))
            .flat_map(|p| p.targets.iter().map(|t| t.name.replace('-', "_")))
            .collect()
    }

    pub fn unknown_packages(&self, selected: &[String]) -> Vec<String> {
        let known: Vec<&str> = self.packages.iter().map(|p| p.name.as_str()).collect();
        selected
            .iter()
            .filter(|name| !known.contains(&name.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Metadata {
        serde_json::from_str(
            r#"{"workspace_root":"/w","target_directory":"/w/target","packages":[
                {"name":"my-app","targets":[{"name":"my-app"},{"name":"tool"}]},
                {"name":"lib2","targets":[{"name":"lib2"}]}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn crate_names_replace_dashes() {
        assert_eq!(sample().crate_names(&[]), ["my_app", "tool", "lib2"]);
    }

    #[test]
    fn package_selection_filters() {
        assert_eq!(sample().crate_names(&["lib2".into()]), ["lib2"]);
        assert_eq!(sample().unknown_packages(&["nope".into()]), ["nope"]);
    }
}
