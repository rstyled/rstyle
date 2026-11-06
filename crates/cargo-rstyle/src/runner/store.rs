//! A run is staged beside where it will live and published whole once it's accepted, so a
//! rejected or failed run leaves nothing a later reader could mistake for a result. Every
//! language's runs live in the project's `.rstyle/runs`, and each keeps its newest few.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STAGING_PREFIX: &str = ".staging-";
const REMOVING_PREFIX: &str = ".pruning-";
/// Staging older than this was left by a process that died; no run takes this long.
const ABANDONED_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// A project's saved runs of one language.
pub(super) struct Runs {
    parent: PathBuf,
    language: &'static str,
    /// How many to keep; `None` keeps them all.
    keep: Option<usize>,
}

/// Where a finished run goes.
pub(super) enum Destination {
    /// A new timestamped directory among the project's runs.
    Runs(Runs),
    /// Exactly this new directory, from `--out`. It's the caller's to manage.
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
            Destination::Runs(runs) => runs.parent.clone(),
            Destination::Exact(out) => {
                require_new(out)?;
                crate::cpg::ingest::parent(out).to_path_buf()
            }
        };
        std::fs::create_dir_all(&parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
        let directory = tempfile::Builder::new()
            .prefix(STAGING_PREFIX)
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

    /// Moves the finished run into place, removes runs it makes surplus, and says where it is.
    // rstyle-allow(side_effect_density): IO boundary: renames the finished run into place
    pub(super) fn publish(self) -> Result<PathBuf, String> {
        match &self.destination {
            Destination::Runs(runs) => {
                let published = publish_run(self.path(), runs, SystemTime::now())?;
                runs.prune(&published);
                Ok(published)
            }
            Destination::Exact(out) => {
                require_new(out)?;
                std::fs::rename(self.path(), out)
                    .map_err(|error| format!("Cannot publish {}: {error}", out.display()))?;
                Ok(out.clone())
            }
        }
    }
}

/// The project's runs of `language`. The folder ignores itself, so a project needs no change to
/// keep it out of version control.
// rstyle-allow(side_effect_density): IO boundary: creates the run folder and its ignore file
pub(super) fn project_runs(project: &Path, language: &'static str) -> Result<Runs, String> {
    let keep = crate::compiler::capture::saved_run_limit()?;
    let folder = project.join(".rstyle");
    let parent = folder.join("runs");
    std::fs::create_dir_all(&parent).map_err(|error| format!("{}: {error}", parent.display()))?;

    let ignore = folder.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").map_err(|error| format!("{}: {error}", ignore.display()))?;
    }
    Ok(Runs {
        parent,
        language,
        keep,
    })
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
fn publish_run(staged: &Path, runs: &Runs, now: SystemTime) -> Result<PathBuf, String> {
    let name = format!("{}-{}", timestamp(now)?, runs.language);
    for attempt in 0..1000 {
        let target = match attempt {
            0 => runs.parent.join(&name),
            _ => runs.parent.join(format!("{name}-{attempt}")),
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
        runs.parent.display()
    ))
}

impl Runs {
    /// A cleanup failure leaves the run itself sound, so it's reported rather than fatal.
    fn prune(&self, published: &Path) {
        if let Err(error) = self.prune_to(published) {
            eprintln!("rstyle: cannot remove old saved runs: {error}");
        }
    }

    // rstyle-allow(side_effect_density): IO boundary: lists a project's runs and removes expired ones
    fn prune_to(&self, published: &Path) -> Result<(), String> {
        let mut saved = Vec::new();
        let mut failures = Vec::new();
        let entries = std::fs::read_dir(&self.parent).map_err(|error| error.to_string())?;

        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with(REMOVING_PREFIX) || abandoned(name, &path) {
                failures.extend(remove(&path).err());
            } else if let Some(key) = run_key(name, self.language) {
                saved.push((key, path));
            }
        }
        let expired = expired(saved, self.keep);
        let retired = expired
            .iter()
            .filter(|run| *run != published)
            .map(|run| retire(run));
        failures.extend(retired.filter_map(Result::err));
        match failures.is_empty() {
            true => Ok(()),
            false => Err(failures.join("; ")),
        }
    }
}

/// Which runs are past the limit, oldest first.
fn expired(mut saved: Vec<(RunKey, PathBuf)>, keep: Option<usize>) -> Vec<PathBuf> {
    let Some(keep) = keep else {
        return Vec::new();
    };
    saved.sort();
    saved.truncate(saved.len().saturating_sub(keep));

    saved.into_iter().map(|(_, run)| run).collect()
}

/// When a run was published, then its counter within that second.
type RunKey = (String, u32);

/// A run of `language`, by its name: anything else in the folder isn't ours to remove.
fn run_key(name: &str, language: &str) -> Option<RunKey> {
    let (time, rest) = name.split_once('-')?;
    let shaped = time.len() == 16
        && time.as_bytes()[8] == b'T'
        && time.ends_with('Z')
        && time
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 15) || byte.is_ascii_digit());
    if !shaped {
        return None;
    }

    let counter = match rest.strip_prefix(language)? {
        "" => 0,
        counter => counter.strip_prefix('-')?.parse().ok()?,
    };
    Some((time.to_string(), counter))
}

fn abandoned(name: &str, path: &Path) -> bool {
    name.starts_with(STAGING_PREFIX)
        && path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > ABANDONED_AFTER)
}

/// Renamed first, so a removal cut short never looks like a run.
// rstyle-allow(side_effect_density): IO boundary: renames a run away, then removes it
fn retire(run: &Path) -> Result<(), String> {
    let name = run.file_name().ok_or("Saved run has no name")?;
    let mut removing = std::ffi::OsString::from(REMOVING_PREFIX);
    removing.push(name);
    let removing = run.with_file_name(removing);

    match std::fs::rename(run, &removing) {
        Ok(()) => remove(&removing),
        // Another run got there first.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", run.display())),
    }
}

fn remove(directory: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", directory.display())),
    }
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

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn runs(dir: &Path, language: &'static str, keep: Option<usize>) -> Runs {
        Runs {
            parent: dir.to_path_buf(),
            language,
            keep,
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn folders(dir: &Path, names: &[&str]) {
        for name in names {
            std::fs::create_dir_all(dir.join(name)).unwrap();
        }
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
        let destination = Destination::Runs(runs(dir.path(), "go", None));
        let staged = Staged::new(destination).unwrap();
        std::fs::write(staged.path().join("partial"), "x").unwrap();
        drop(staged);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn runs_within_one_second_get_their_own_folders() {
        let dir = tempfile::tempdir().unwrap();
        let saved = runs(dir.path(), "go", None);
        for _ in 0..3 {
            let staged = tempfile::tempdir_in(dir.path()).unwrap();
            std::fs::write(staged.path().join("report"), "x").unwrap();
            publish_run(staged.path(), &saved, at(0)).unwrap();
        }
        assert_eq!(
            names(dir.path()),
            [
                "19700101T000000Z-go",
                "19700101T000000Z-go-1",
                "19700101T000000Z-go-2"
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
        let saved = project_runs(dir.path(), "go").unwrap();
        assert_eq!(saved.parent, dir.path().join(".rstyle/runs"));
        let ignore = dir.path().join(".rstyle/.gitignore");
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "*\n");
        std::fs::write(&ignore, "edited\n").unwrap();
        project_runs(dir.path(), "go").unwrap();
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "edited\n");
    }

    #[test]
    fn a_name_is_a_run_only_if_it_is_shaped_like_one_of_this_language() {
        let key = |name| run_key(name, "go");
        assert_eq!(
            key("20261106T075815Z-go"),
            Some(("20261106T075815Z".into(), 0))
        );
        assert_eq!(
            key("20261106T075815Z-go-3"),
            Some(("20261106T075815Z".into(), 3))
        );
        for other in [
            "20261106T075815Z-rust",
            "20261106T075815Z-gopher",
            "20261106T075815Z-go-x",
            "20261106T075815Z",
            "2026110ZT075815Z-go",
            "notes-go",
            "my-go",
        ] {
            assert_eq!(key(other), None, "{other}");
        }
    }

    #[test]
    fn pruning_keeps_the_newest_of_one_language_and_nothing_else_changes() {
        let dir = tempfile::tempdir().unwrap();
        folders(
            dir.path(),
            &[
                "20260101T000000Z-go",
                "20260102T000000Z-go",
                "20260103T000000Z-go",
                "20260103T000000Z-go-1",
                "20260101T000000Z-rust",
                "notes",
            ],
        );
        let newest = dir.path().join("20260103T000000Z-go-1");
        runs(dir.path(), "go", Some(2)).prune(&newest);
        assert_eq!(
            names(dir.path()),
            [
                "20260101T000000Z-rust",
                "20260103T000000Z-go",
                "20260103T000000Z-go-1",
                "notes"
            ]
        );
    }

    #[test]
    fn keeping_them_all_removes_none() {
        let dir = tempfile::tempdir().unwrap();
        folders(dir.path(), &["20260101T000000Z-go", "20260102T000000Z-go"]);
        runs(dir.path(), "go", None).prune(&dir.path().join("20260102T000000Z-go"));
        assert_eq!(names(dir.path()).len(), 2);
    }

    #[test]
    fn the_run_just_published_survives_a_clock_that_went_backwards() {
        let dir = tempfile::tempdir().unwrap();
        folders(dir.path(), &["20260101T000000Z-go", "20260102T000000Z-go"]);
        let published = dir.path().join("20260101T000000Z-go");
        runs(dir.path(), "go", Some(1)).prune(&published);
        assert!(published.exists());
    }

    #[test]
    fn an_interrupted_removal_is_finished_and_live_staging_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        folders(
            dir.path(),
            &[".pruning-20260101T000000Z-go", ".staging-live"],
        );
        runs(dir.path(), "go", Some(1)).prune(&dir.path().join("none"));
        assert_eq!(names(dir.path()), [".staging-live"]);
    }
}
