//! The threads rustc runs its front end on, and the driver writes a crate's documents on.
use std::num::NonZeroU8;

/// The compiler's arguments with rustc's front end on `threads`, when that's more than one and
/// the invocation doesn't set them itself: rustc refuses `-Zthreads` beside `--jobs-frontend`,
/// or above `--jobs`.
pub fn compiler_arguments(mut arguments: Vec<String>, threads: NonZeroU8) -> Vec<String> {
    if threads.get() > 1 && !sets_threads(&arguments) {
        arguments.push(format!("-Zthreads={threads}"));
    }
    arguments
}

fn sets_threads(arguments: &[String]) -> bool {
    let separate = arguments
        .windows(2)
        .any(|pair| pair[0] == "-Z" && pair[1].starts_with("threads"));
    separate
        || arguments.iter().any(|argument| {
            ["-Zthreads", "--jobs", "-j"]
                .iter()
                .any(|option| argument.starts_with(option))
        })
}

/// Whether the crate's documents are built on rustc's threads: only while its front end runs
/// on more than one, which only `-Zthreads` above 1 asks for. Otherwise they're built in order
/// on this thread, each written before the next is built.
pub fn parallel() -> bool {
    rustc_data_structures::sync::is_dyn_thread_safe()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_driver::{Callbacks, Compilation};
    use rustc_interface::interface::Compiler;
    use rustc_middle::ty::TyCtxt;

    /// Set in a probe's own process: the threads it compiles on.
    const PROBE: &str = "RSTYLE_DRIVER_THREADS_PROBE";

    fn on(threads: u8, given: &[&str]) -> Vec<String> {
        let given = given.iter().map(|argument| argument.to_string()).collect();
        compiler_arguments(given, NonZeroU8::new(threads).unwrap())
    }

    #[test]
    fn one_thread_adds_no_option_and_more_add_rustcs_own() {
        assert_eq!(on(1, &["rustc", "lib.rs"]), ["rustc", "lib.rs"]);
        assert_eq!(
            on(6, &["rustc", "lib.rs"]),
            ["rustc", "lib.rs", "-Zthreads=6"]
        );
        // Cargo passes every compilation `--json`, which sets no jobs.
        let cargo = ["rustc", "--json=diagnostic-rendered-ansi", "lib.rs"];
        assert_eq!(on(6, &cargo).last().unwrap(), "-Zthreads=6");
    }

    #[test]
    fn an_invocation_that_sets_rustcs_threads_keeps_them() {
        for set in [
            &["-Zthreads=2"][..],
            &["-Z", "threads=2"],
            &["--jobs-frontend=2"],
            &["-Zunstable-options", "--jobs", "2"],
            &["-Zunstable-options", "-j2"],
        ] {
            let given: Vec<&str> = ["rustc", "lib.rs"]
                .into_iter()
                .chain(set.iter().copied())
                .collect();
            assert_eq!(on(6, &given), given, "{set:?}");
        }
    }

    struct Probe(Option<bool>);

    impl Callbacks for Probe {
        fn after_analysis<'tcx>(&mut self, _: &Compiler, _: TyCtxt<'tcx>) -> Compilation {
            self.0 = Some(parallel());
            Compilation::Stop
        }
    }

    /// Compiles a one-function crate with what the driver gives rustc for `threads`, and prints
    /// whether its documents would be built in parallel.
    fn probe(threads: u8) {
        let directory = std::env::temp_dir().join(format!("rstyle-probe-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("lib.rs");
        std::fs::write(&source, "pub fn one() -> u8 { 1 }\n").unwrap();
        let (source, out) = (source.to_str().unwrap(), directory.to_str().unwrap());
        let given = [
            "rustc",
            source,
            "--crate-type=lib",
            "--emit=metadata",
            "--out-dir",
            out,
        ];
        let arguments = on(threads, &given);
        println!("arguments={}", arguments.join(" "));

        let mut probe = Probe(None);
        rustc_driver::run_compiler(&arguments, &mut probe);
        std::fs::remove_dir_all(&directory).unwrap();
        println!("parallel={}", probe.0.expect("the crate was analyzed"));
    }

    /// What a probe on `threads` reported: the arguments rustc got, and whether the documents
    /// would be built in parallel. Each probe has a process of its own, since rustc chooses once
    /// per process whether it runs in parallel.
    fn probed(threads: u8) -> (String, bool) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "threads::tests::the_off_switch_compiles_and_writes_on_one_thread",
                "--exact",
                "--nocapture",
            ])
            .env(PROBE, threads.to_string())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}{stderr}");
        let field = |name: &str| {
            let prefix = format!("{name}=");
            let value = stdout
                .lines()
                .find_map(|line| line.strip_prefix(prefix.as_str()));
            assert!(value.is_some(), "no {name} in {stdout}{stderr}");
            value.unwrap_or_default().to_owned()
        };
        (field("arguments"), field("parallel") == "true")
    }

    #[test]
    fn the_off_switch_compiles_and_writes_on_one_thread() {
        if let Ok(threads) = std::env::var(PROBE) {
            return probe(threads.parse().unwrap());
        }
        let (arguments, parallel) = probed(1);
        assert!(!arguments.contains("-Zthreads"), "{arguments}");
        assert!(!parallel);

        let (arguments, parallel) = probed(3);
        assert!(arguments.ends_with(" -Zthreads=3"), "{arguments}");
        assert!(parallel);
    }
}
