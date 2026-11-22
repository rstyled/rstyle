//! An owner index read where it wasn't written: a build directory restored into a checkout at
//! another path, as a CI cache or a fresh worker restores one.
//!
//! The index records its plan, the artifacts Cargo reported and its units as that checkout named
//! them, with its absolute paths. Read in place, nothing changes. Read in another checkout, each
//! of those paths is named by placeholder as it was written, then put back as this checkout names
//! it, so an index that captured the same source still serves it. The units' documents need no
//! such change: they hold no path of the machine.
use super::{Capture, Plan, Target, Unit, build_lane};
use rstyle_capture::Places;
use rstyle_capture::places::map_strings;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) struct Relocation {
    /// Where the index was written, and where it's read.
    from: Places,
    to: Places,
    runs: (PathBuf, PathBuf),
}

impl Relocation {
    /// From the runs directory an index was written in, with its plan, to this capture's.
    pub fn new(runs: &Path, plan: &Plan, capture: &Capture) -> Result<Self, String> {
        let here = capture
            .directory
            .parent()
            .ok_or("Missing capture parent")?
            .to_path_buf();
        Ok(Self {
            from: places(plan, &runs.join(build_lane::NAME)),
            to: capture.places(),
            runs: (runs.to_path_buf(), here),
        })
    }

    /// The text as this checkout names what it named.
    pub fn text(&self, text: &str) -> String {
        self.to.local(&self.from.portable(text))
    }

    pub fn value(&self, value: &Value) -> Value {
        map_strings(value, &|text| self.text(text))
    }

    pub fn plan(&self, plan: &Plan) -> Result<Plan, String> {
        self.typed(plan)
    }

    pub fn target(&self, target: &Target) -> Result<Target, String> {
        self.typed(target)
    }

    /// A unit's directory, in this checkout's runs directory.
    pub fn directory(&self, directory: &Path) -> PathBuf {
        let (from, to) = &self.runs;
        directory
            .strip_prefix(from)
            .map_or_else(|_| directory.to_path_buf(), |rest| to.join(rest))
    }

    /// The unit as this checkout would have captured it: its target, invocation and directory
    /// named as this checkout names them.
    pub fn unit(&self, unit: &Unit) -> Result<Unit, String> {
        Ok(Unit {
            target: self.target(&unit.target)?,
            invocation: unit.invocation.iter().map(|part| self.text(part)).collect(),
            directory: self.directory(&unit.directory),
            ..unit.clone()
        })
    }

    fn typed<T: Serialize + DeserializeOwned>(&self, value: &T) -> Result<T, String> {
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        serde_json::from_value(self.value(&value)).map_err(|error| error.to_string())
    }
}

/// The directories a plan's checkout named: its root, its build directory and its sysroot.
fn places(plan: &Plan, build: &Path) -> Places {
    let sysroot = (!plan.sysroot.as_os_str().is_empty()).then_some(plan.sysroot.as_path());
    Places::new(&plan.root, Some(build), sysroot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_outside_the_runs_stays_where_it_is() {
        let relocation = Relocation {
            from: Places::new(Path::new("/old"), None, None),
            to: Places::new(Path::new("/new"), None, None),
            runs: (
                "/old/target/rstyle/runs".into(),
                "/new/target/rstyle/runs".into(),
            ),
        };
        assert_eq!(
            relocation.directory(Path::new("/old/target/rstyle/runs/run-a/invocation-b")),
            Path::new("/new/target/rstyle/runs/run-a/invocation-b")
        );
        assert_eq!(
            relocation.directory(Path::new("/elsewhere/invocation-b")),
            Path::new("/elsewhere/invocation-b")
        );
        assert_eq!(
            relocation.text("path+file:///old/crates/a#0.1.0"),
            "path+file:///new/crates/a#0.1.0"
        );
    }
}
