use super::*;
use std::{
    fs::File,
    time::{Duration, SystemTime},
};

struct Runs(tempfile::TempDir);

impl Runs {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn parent(&self) -> &Path {
        self.0.path()
    }

    /// A run last used `minutes` ahead of now, which outranks the directory's own time.
    fn add(&self, name: &str, minutes: u64) -> PathBuf {
        let run = self.parent().join(name);
        std::fs::create_dir(&run).unwrap();
        let report = File::create(run.join("report.json")).unwrap();
        report
            .set_modified(SystemTime::now() + Duration::from_secs(minutes * 60))
            .unwrap();
        run
    }

    fn names(&self) -> BTreeSet<String> {
        std::fs::read_dir(self.parent())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| !name.starts_with("advisory-build"))
            .collect()
    }
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(ToString::to_string).collect()
}

#[test]
fn a_limit_is_a_positive_count_or_all() {
    assert_eq!(parsed_limit(None), Ok(Some(KEPT_RUNS)));
    assert_eq!(parsed_limit(Some("all")), Ok(None));
    assert_eq!(parsed_limit(Some("2")), Ok(Some(2)));

    for invalid in ["0", "-1", "many", ""] {
        let error = parsed_limit(Some(invalid)).unwrap_err();
        assert!(error.contains(LIMIT_VARIABLE), "{error}");
    }
}

#[test]
fn the_latest_used_runs_stay() {
    let runs = Runs::new();
    runs.add("run-oldest", 10);
    runs.add("run-older", 20);
    runs.add("run-newer", 30);
    runs.add("run-newest", 40);

    prune_to(runs.parent(), 2).unwrap();

    assert_eq!(runs.names(), names(&["run-newer", "run-newest"]));
}

#[test]
fn a_replay_keeps_its_run_recent_though_the_directory_is_older() {
    let runs = Runs::new();
    let replayed = runs.add("run-replayed", 10);
    runs.add("run-other", 20);
    File::create(replayed.join("report.json"))
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(30 * 60))
        .unwrap();

    prune_to(runs.parent(), 1).unwrap();

    assert_eq!(runs.names(), names(&["run-replayed"]));
}

#[test]
fn a_run_in_use_stays_and_goes_once_released() {
    let runs = Runs::new();
    let used = runs.add("run-used", 10);
    runs.add("run-newest", 20);
    let lock = build_lane::open_lock(used.join(LOCK_FILE)).unwrap();
    lock.lock_shared().unwrap();

    prune_to(runs.parent(), 1).unwrap();
    assert_eq!(runs.names(), names(&["run-used", "run-newest"]));

    drop(lock);
    prune_to(runs.parent(), 1).unwrap();
    assert_eq!(runs.names(), names(&["run-newest"]));
}

#[test]
fn a_new_run_is_held_against_a_later_limit() {
    let runs = Runs::new();
    let held = new_run(runs.parent()).unwrap();
    runs.add("run-newest", 20);

    prune_to(runs.parent(), 1).unwrap();

    assert!(held.is_dir());
}

#[test]
fn a_shared_build_defers_pruning() {
    let runs = Runs::new();
    runs.add("run-older", 10);
    runs.add("run-newest", 20);
    let lane = build_lane::try_acquire(runs.parent()).unwrap().unwrap();

    prune_to(runs.parent(), 1).unwrap();
    assert_eq!(runs.names(), names(&["run-older", "run-newest"]));

    drop(lane);
    prune_to(runs.parent(), 1).unwrap();
    assert_eq!(runs.names(), names(&["run-newest"]));
}

#[test]
fn a_run_the_owner_index_names_is_not_expired() {
    let older = (SystemTime::UNIX_EPOCH, PathBuf::from("runs/run-indexed"));
    let newer = (SystemTime::now(), PathBuf::from("runs/run-newest"));
    let indexed = BTreeSet::from([OsString::from("run-indexed")]);

    assert!(expired(vec![older, newer], 1, &indexed).is_empty());
}

#[test]
fn a_removal_cut_short_is_finished_by_the_next_capture() {
    let runs = Runs::new();
    let interrupted = runs.add("pruning-run-gone", 10);
    std::fs::write(interrupted.join("document.cpg.json"), "{}").unwrap();
    runs.add("run-newest", 20);

    prune_to(runs.parent(), 5).unwrap();

    assert_eq!(runs.names(), names(&["run-newest"]));
}

#[test]
fn only_run_directories_are_removed() {
    let runs = Runs::new();
    std::fs::write(runs.parent().join("run-file"), "kept").unwrap();
    std::fs::write(runs.parent().join("advisory-owners.json"), "kept").unwrap();
    std::fs::create_dir(runs.parent().join("other")).unwrap();
    runs.add("run-oldest", 10);
    runs.add("run-newest", 20);

    prune_to(runs.parent(), 1).unwrap();

    assert_eq!(
        runs.names(),
        names(&["run-file", "advisory-owners.json", "other", "run-newest"])
    );
}

#[test]
fn no_limit_removes_nothing() {
    let runs = Runs::new();
    runs.add("run-older", 10);
    runs.add("run-newest", 20);

    prune(runs.parent(), None);

    assert_eq!(runs.names(), names(&["run-older", "run-newest"]));
}
