//! Working out a C or C++ project's compilation database: one it already has, or else one its
//! build writes. A run that ends up with neither still goes ahead on guessed flags, which the
//! frontend's report declares, so this warns and says how to specify one.
//!
//! Asking the build runs its scripts, and for Make the build itself, so a flag can say not to.

use super::store;
use serde::de::IgnoredAny;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DATABASE: &str = "compile_commands.json";
/// Under the project's `.rstyle`, so its own build directory stays as it was.
const GENERATED_FOLDER: &str = "compile-database";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Build {
    CMake,
    Meson,
    Make,
}

impl Build {
    /// The build `source` holds. A Makefile often only wraps one of the others, so it comes last.
    fn detect(source: &Path) -> Option<Self> {
        let holds = |name: &str| source.join(name).is_file();
        if holds("CMakeLists.txt") {
            return Some(Self::CMake);
        }
        if holds("meson.build") {
            return Some(Self::Meson);
        }
        ["GNUmakefile", "makefile", "Makefile"]
            .into_iter()
            .any(holds)
            .then_some(Self::Make)
    }

    fn name(self) -> &'static str {
        match self {
            Self::CMake => "CMake",
            Self::Meson => "Meson",
            Self::Make => "Make",
        }
    }

    /// The program that writes a database into `folder`, run from the project's directory.
    fn invocation(self, folder: &Path) -> (&'static str, Vec<OsString>) {
        let build = folder.as_os_str().to_os_string();
        match self {
            Self::CMake => (
                "cmake",
                vec![
                    "-S".into(),
                    ".".into(),
                    "-B".into(),
                    build,
                    "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".into(),
                ],
            ),
            Self::Meson => ("meson", vec!["setup".into(), build]),
            // An up-to-date Make compiles nothing, so a later run keeps what an earlier one saw.
            Self::Make => (
                "bear",
                vec![
                    "--append".into(),
                    "--output".into(),
                    folder.join(DATABASE).into_os_string(),
                    "--".into(),
                    "make".into(),
                ],
            ),
        }
    }
}

/// Whether `directory` has a database where the frontend looks without being told.
pub(super) fn found_in(directory: &Path) -> bool {
    directory.join(DATABASE).is_file() || directory.join("build").join(DATABASE).is_file()
}

/// Whether a folder's name says a build wrote it. A vendored project can carry a database of its
/// own, and using that would read only its files, so any other folder isn't considered.
fn named_like_a_build(name: &str) -> bool {
    matches!(name, "builddir" | "_build" | "out")
        || name.starts_with("build")
        || name.starts_with("cmake-build")
}

/// The folders under `source` that look like builds and hold a database, in a stable order.
// rstyle-allow(side_effect_density): IO boundary: lists the project's top folder
fn build_directories(source: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(source) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| named_like_a_build(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .filter(|path| path.join(DATABASE).is_file())
        .collect();
    found.sort();
    found
}

/// The database in a build directory the frontend wouldn't look in, and a note saying which.
// rstyle-allow(side_effect_density): IO boundary: says which database it chose
fn in_build_directory(source: &Path) -> Option<PathBuf> {
    let found = build_directories(source);
    let (chosen, others) = found.split_first()?;

    let also = match others {
        [] => String::new(),
        others => {
            let names: Vec<String> = others.iter().map(|it| it.display().to_string()).collect();
            format!(" (also found {})", names.join(", "))
        }
    };
    eprintln!(
        "rstyle: using the {DATABASE} in {}{also}; --compilation-database names another",
        chosen.display()
    );
    Some(chosen.clone())
}

/// A database for a run told not to generate one, if the frontend needs telling where it is.
/// Finding none is a warning, not a failure.
// rstyle-allow(side_effect_density): IO boundary: looks for a database and reports what it found
pub(super) fn detected(source: &Path) -> Option<PathBuf> {
    if found_in(source) {
        return None;
    }
    if let Some(found) = in_build_directory(source) {
        return Some(found);
    }
    if let Some(earlier) = generated_earlier(source) {
        eprintln!(
            "rstyle: using the {DATABASE} generated earlier in {}",
            earlier.display()
        );
        return Some(earlier);
    }

    let build = Build::detect(source);
    let reason = match build {
        Some(build) => format!(
            "no {DATABASE} in {}, which looks like a {} project",
            source.display(),
            build.name()
        ),
        None => format!("no {DATABASE} in {}, and {UNKNOWN_BUILD}", source.display()),
    };
    warn(&reason, source, build, true);
    None
}

/// A database from the project's build, unless it already has one. A build that can't be worked
/// out or doesn't write one is a warning, not a failure.
// rstyle-allow(side_effect_density): process boundary: runs the project's build
pub(super) fn generated(source: &Path) -> Option<PathBuf> {
    if found_in(source) {
        return None;
    }
    if let Some(found) = in_build_directory(source) {
        return Some(found);
    }
    let Some(build) = Build::detect(source) else {
        let reason = format!("couldn't generate a {DATABASE}: {UNKNOWN_BUILD}");
        warn(&reason, source, None, false);
        return None;
    };
    match run_build(build, source) {
        Ok(folder) => Some(folder),
        Err(reason) => fall_back(&reason, source, build),
    }
}

/// A project mid-change often doesn't build, and what it last wrote beats guessing flags.
// rstyle-allow(side_effect_density): IO boundary: says what the run falls back on
fn fall_back(reason: &str, source: &Path, build: Build) -> Option<PathBuf> {
    let Some(earlier) = generated_earlier(source) else {
        warn(reason, source, Some(build), false);
        return None;
    };
    eprintln!(
        "rstyle: warning: {reason}\nUsing what's in {}, which may be out of date",
        earlier.display()
    );
    Some(earlier)
}

/// The folder of a database an earlier run generated, if it lists anything.
// rstyle-allow(side_effect_density): IO boundary: reads what an earlier run wrote
fn generated_earlier(source: &Path) -> Option<PathBuf> {
    let folder = source.join(store::PROJECT_FOLDER).join(GENERATED_FOLDER);
    let listed = entries(&folder.join(DATABASE)).ok()?;
    (listed > 0).then_some(folder)
}

const UNKNOWN_BUILD: &str =
    "there's no CMakeLists.txt, meson.build or Makefile to say how it builds";

// rstyle-allow(side_effect_density): process boundary: runs the project's build
fn run_build(build: Build, source: &Path) -> Result<PathBuf, String> {
    let folder = store::project_folder(source)?.join(GENERATED_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;

    let (program, arguments) = build.invocation(&folder);
    eprintln!(
        "rstyle: generating {DATABASE} with {program}, which runs this project's build scripts; \
         --no-generate-database skips this"
    );
    let status = Command::new(program)
        .args(&arguments)
        .current_dir(source)
        .stdin(Stdio::null())
        .stdout(Stdio::from(std::io::stderr()))
        .status()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => format!(
                "couldn't generate a {DATABASE}: {program} isn't on PATH, and this {} project \
                 needs it",
                build.name()
            ),
            _ => format!("couldn't generate a {DATABASE}: cannot run {program}: {error}"),
        })?;
    if !status.success() {
        return Err(format!(
            "couldn't generate a {DATABASE}: {program} failed with {status}"
        ));
    }
    let database = folder.join(DATABASE);
    match entries(&database) {
        Ok(0) => Err(format!(
            "{} lists nothing to compile. A Make build that's up to date compiles nothing, so \
             clean it and try again",
            database.display()
        )),
        Ok(_) => Ok(folder),
        Err(error) => Err(format!(
            "couldn't generate a {DATABASE}: {program} didn't write a usable one: {error}"
        )),
    }
}

/// How many entries a database lists.
// rstyle-allow(side_effect_density): IO boundary: reads a database
fn entries(path: &Path) -> Result<usize, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let listed: Vec<IgnoredAny> =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(listed.len())
}

// rstyle-allow(side_effect_density): IO boundary: prints a warning
fn warn(reason: &str, source: &Path, build: Option<Build>, offer_generating: bool) {
    eprintln!(
        "rstyle: warning: {reason}\n{}",
        instructions(source, build, offer_generating)
    );
}

/// How to specify a database. The commands a build offers write one where the frontend looks.
fn instructions(source: &Path, build: Option<Build>, offer_generating: bool) -> String {
    let command = match build {
        Some(Build::CMake) => "cmake -S . -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        Some(Build::Meson) => "meson setup build",
        Some(Build::Make) => "bear -- make",
        None => "bear -- <your build command>",
    };
    let generating = match (offer_generating, build) {
        (true, Some(build)) => format!(
            "\n  rstyle cxx\n      without --no-generate-database, asks this {} build for one, \
             which runs its build scripts",
            build.name()
        ),
        _ => String::new(),
    };
    format!(
        "The frontend will guess compile flags, and lose what it can't parse. To specify a \
         database:{generating}\n  {command}\n      run in {}, then run rstyle cxx again\n  rstyle \
         cxx --compilation-database PATH\n      names one you already have",
        source.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(names: &[&str]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        for name in names {
            std::fs::write(directory.path().join(name), "").unwrap();
        }
        directory
    }

    #[test]
    fn a_build_is_known_by_the_file_it_starts_from() {
        let detected = |names: &[&str]| Build::detect(holding(names).path());
        assert_eq!(detected(&["CMakeLists.txt"]), Some(Build::CMake));
        assert_eq!(detected(&["meson.build"]), Some(Build::Meson));
        for makefile in ["Makefile", "makefile", "GNUmakefile"] {
            assert_eq!(detected(&[makefile]), Some(Build::Make));
        }
        assert_eq!(detected(&["main.cpp"]), None);
    }

    #[test]
    fn a_makefile_beside_another_build_only_wraps_it() {
        let detected = |names: &[&str]| Build::detect(holding(names).path());
        assert_eq!(
            detected(&["Makefile", "CMakeLists.txt"]),
            Some(Build::CMake)
        );
        assert_eq!(detected(&["Makefile", "meson.build"]), Some(Build::Meson));
    }

    #[test]
    fn a_directory_named_like_a_build_file_is_not_one() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("Makefile")).unwrap();
        assert_eq!(Build::detect(directory.path()), None);
    }

    #[test]
    fn a_database_is_found_where_the_frontend_looks() {
        let directory = holding(&[]);
        assert!(!found_in(directory.path()));
        std::fs::create_dir(directory.path().join("build")).unwrap();
        assert!(!found_in(directory.path()));
        std::fs::write(directory.path().join("build").join(DATABASE), "[]").unwrap();
        assert!(found_in(directory.path()));
    }

    #[test]
    fn only_folders_named_like_builds_are_searched() {
        for name in [
            "build-debug",
            "cmake-build-release",
            "builddir",
            "_build",
            "out",
        ] {
            assert!(named_like_a_build(name), "{name}");
        }
        for name in ["third_party", "vendor", "src", "output", ".rstyle"] {
            assert!(!named_like_a_build(name), "{name}");
        }
    }

    #[test]
    fn build_directories_are_found_in_a_stable_order_and_need_a_database() {
        let directory = holding(&[]);
        for name in ["cmake-build-release", "cmake-build-debug", "out", "vendor"] {
            std::fs::create_dir(directory.path().join(name)).unwrap();
        }
        for name in ["cmake-build-release", "cmake-build-debug", "vendor"] {
            std::fs::write(directory.path().join(name).join(DATABASE), "[]").unwrap();
        }
        assert_eq!(
            build_directories(directory.path()),
            [
                directory.path().join("cmake-build-debug"),
                directory.path().join("cmake-build-release"),
            ]
        );
    }

    #[test]
    fn instructions_fit_the_build_that_was_guessed() {
        let source = Path::new("/project");
        let cmake = instructions(source, Some(Build::CMake), true);
        assert!(cmake.contains("without --no-generate-database"), "{cmake}");
        assert!(cmake.contains("asks this CMake build"), "{cmake}");
        assert!(cmake.contains("cmake -S . -B build"), "{cmake}");
        assert!(cmake.contains("--compilation-database PATH"), "{cmake}");
        assert!(cmake.contains("/project"), "{cmake}");

        assert!(instructions(source, Some(Build::Meson), true).contains("meson setup build"));
        assert!(instructions(source, Some(Build::Make), true).contains("bear -- make"));
    }

    #[test]
    fn instructions_offer_generating_only_if_it_could_help() {
        let source = Path::new("/project");
        let unknown = instructions(source, None, true);
        assert!(!unknown.contains("--no-generate-database"), "{unknown}");
        assert!(
            unknown.contains("bear -- <your build command>"),
            "{unknown}"
        );
        assert!(unknown.contains("--compilation-database PATH"), "{unknown}");

        let failed = instructions(source, Some(Build::CMake), false);
        assert!(!failed.contains("--no-generate-database"), "{failed}");
        assert!(failed.contains("cmake -S . -B build"), "{failed}");
    }
}
