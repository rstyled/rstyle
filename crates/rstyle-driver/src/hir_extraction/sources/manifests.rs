//! Manifest snapshots belong to one compiler capture, never a global cache.
use rstyle_pdg::SourceSnapshot;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct Manifests(BTreeMap<PathBuf, SourceSnapshot>);

impl Manifests {
    pub fn read(&mut self, path: &Path) -> Result<SourceSnapshot, String> {
        self.load(path, || {
            std::fs::read_to_string(path).map_err(|error| error.to_string())
        })
    }

    fn load(
        &mut self,
        path: &Path,
        read: impl FnOnce() -> Result<String, String>,
    ) -> Result<SourceSnapshot, String> {
        if let Some(snapshot) = self.0.get(path) {
            return Ok(snapshot.clone());
        }

        let snapshot: SourceSnapshot = read()?.into();
        self.0.insert(path.to_owned(), snapshot.clone());
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_reads_and_actual_hashes_are_capture_local_and_once_per_owner() {
        let path = Path::new("/sample/Cargo.toml");
        let mut capture = Manifests::default();
        let first = capture
            .load(path, || Ok("[package]\nname='sample'".into()))
            .unwrap();
        let repeated = capture
            .load(path, || panic!("immutable manifest was reread"))
            .unwrap();
        assert!(SourceSnapshot::ptr_eq(&first, &repeated));
        assert!(std::ptr::eq(first.identity(), repeated.identity()));

        let mut independent = Manifests::default();
        let edited = independent
            .load(path, || Ok("[package]\nname='edited'".into()))
            .unwrap();
        assert!(!SourceSnapshot::ptr_eq(&first, &edited));
        assert_ne!(first.identity(), edited.identity());
        let missing = Path::new("/other/Cargo.toml");
        assert!(capture.load(missing, || Err("missing".into())).is_err());
        let retry = capture.load(missing, || Ok("[package]".into())).unwrap();
        assert_eq!(retry.as_ref(), "[package]");
        assert_eq!(capture.0.len(), 2);
    }
}
