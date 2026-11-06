//! A run is staged beside where it will live and published whole once it's accepted, so a
//! rejected or failed run leaves nothing a later reader could mistake for a result.
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where a finished run goes.
pub(super) enum Destination {
    /// A new timestamped directory under this one.
    Runs(PathBuf),
    /// Exactly this new directory, from `--out`.
    Exact(PathBuf),
}

pub(super) struct Staged {
    directory: tempfile::TempDir,
    destination: Destination,
}

impl Staged {
    // rstyle-allow(side_effect_density): IO boundary: creates the staging directory
    pub(super) fn new(destination: Destination) -> Result<Self, String> {
        let parent = match &destination {
            Destination::Runs(parent) => parent.clone(),
            Destination::Exact(out) => {
                require_new(out)?;
                crate::cpg::ingest::parent(out).to_path_buf()
            }
        };
        std::fs::create_dir_all(&parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
        let directory = tempfile::Builder::new()
            .prefix(".staging-")
            .tempdir_in(&parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
        Ok(Self {
            directory,
            destination,
        })
    }

    pub(super) fn path(&self) -> &Path {
        self.directory.path()
    }

    /// A new folder inside the run.
    pub(super) fn folder(&self, name: &str) -> Result<PathBuf, String> {
        let folder = self.path().join(name);
        std::fs::create_dir_all(&folder)
            .map_err(|error| format!("{}: {error}", folder.display()))?;
        Ok(folder)
    }

    /// Moves the finished run into place and returns where it is.
    // rstyle-allow(side_effect_density): IO boundary: renames the finished run into place
    pub(super) fn publish(self) -> Result<PathBuf, String> {
        match &self.destination {
            Destination::Runs(parent) => publish_run(self.path(), parent, SystemTime::now()),
            Destination::Exact(out) => {
                require_new(out)?;
                std::fs::rename(self.path(), out)
                    .map_err(|error| format!("Cannot publish {}: {error}", out.display()))?;
                Ok(out.clone())
            }
        }
    }
}

/// Go has no `target/`, so its runs get a folder of their own in the project. It ignores
/// itself, so a project needs no change to keep it out of version control.
// rstyle-allow(side_effect_density): IO boundary: creates the run folder and its ignore file
pub(super) fn project_runs(project: &Path) -> Result<PathBuf, String> {
    let folder = project.join(".rstyle");
    let runs = folder.join("runs");
    std::fs::create_dir_all(&runs).map_err(|error| format!("{}: {error}", runs.display()))?;

    let ignore = folder.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").map_err(|error| format!("{}: {error}", ignore.display()))?;
    }
    Ok(runs)
}

fn require_new(out: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(out) {
        Ok(_) => Err(format!("{} must be a new directory", out.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", out.display())),
    }
}

/// Two runs can finish within a second, so a taken name gets a counter.
// rstyle-allow(side_effect_density): IO boundary: renames the finished run into place
fn publish_run(staged: &Path, parent: &Path, now: SystemTime) -> Result<PathBuf, String> {
    let name = timestamp(now)?;
    for attempt in 0..1000 {
        let target = match attempt {
            0 => parent.join(&name),
            _ => parent.join(format!("{name}-{attempt}")),
        };
        if target.exists() {
            continue;
        }
        return std::fs::rename(staged, &target)
            .map(|()| target)
            .map_err(|error| format!("Cannot publish the run: {error}"));
    }
    Err(format!(
        "{} already holds too many runs named {name}",
        parent.display()
    ))
}

/// UTC in a form every filesystem accepts and that sorts by time, such as `20261106T081512Z`.
fn timestamp(now: SystemTime) -> Result<String, String> {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let (year, month, day) = civil_date(seconds / 86_400);
    let time = seconds % 86_400;
    Ok(format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        time / 3600,
        time / 60 % 60,
        time % 60
    ))
}

/// The proleptic Gregorian date of a day count since 1970, by Howard Hinnant's `civil_from_days`.
fn civil_date(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let of_era = shifted % 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);

    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn timestamps_are_utc_and_sort_by_time() {
        assert_eq!(timestamp(at(0)).unwrap(), "19700101T000000Z");
        assert_eq!(timestamp(at(951_782_400)).unwrap(), "20000229T000000Z");
        assert_eq!(timestamp(at(951_868_799)).unwrap(), "20000229T235959Z");
        assert_eq!(timestamp(at(1_793_951_895)).unwrap(), "20261106T075815Z");
    }

    #[test]
    fn a_rejected_run_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let staged = Staged::new(Destination::Runs(dir.path().to_path_buf())).unwrap();
        std::fs::write(staged.path().join("partial"), "x").unwrap();
        drop(staged);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn runs_within_one_second_get_their_own_folders() {
        let dir = tempfile::tempdir().unwrap();
        let mut published = Vec::new();
        for _ in 0..3 {
            let staged = tempfile::tempdir_in(dir.path()).unwrap();
            std::fs::write(staged.path().join("report"), "x").unwrap();
            published.push(publish_run(staged.path(), dir.path(), at(0)).unwrap());
        }
        let names: Vec<_> = published
            .iter()
            .map(|path| path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "19700101T000000Z",
                "19700101T000000Z-1",
                "19700101T000000Z-2"
            ]
        );
    }

    #[test]
    fn an_exact_destination_must_be_new_and_lands_there() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let staged = Staged::new(Destination::Exact(out.clone())).unwrap();
        std::fs::write(staged.path().join("report"), "x").unwrap();
        assert_eq!(staged.publish().unwrap(), out);
        assert!(out.join("report").exists());
        assert!(Staged::new(Destination::Exact(out)).is_err());
    }

    #[test]
    fn a_project_folder_ignores_itself_and_keeps_what_was_edited() {
        let dir = tempfile::tempdir().unwrap();
        let runs = project_runs(dir.path()).unwrap();
        assert_eq!(runs, dir.path().join(".rstyle/runs"));
        let ignore = dir.path().join(".rstyle/.gitignore");
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "*\n");
        std::fs::write(&ignore, "edited\n").unwrap();
        project_runs(dir.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "edited\n");
    }
}
