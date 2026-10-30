use super::*;

#[test]
fn root_style_baseline_flags_require_compiler_style_and_explicit_gate_inputs() {
    for flags in [
        vec!["--baseline", "baseline.json"],
        vec!["--write-baseline", "baseline.json"],
        vec!["--baseline", "baseline.json", "--fail-on-new"],
    ] {
        assert!(
            Cli::try_parse_from(["cargo-rstyle"].into_iter().chain(flags.iter().copied())).is_err()
        );
        for engine in ["legacy", "compiler"] {
            let cli = Cli::try_parse_from(
                ["cargo-rstyle", "--style-engine", engine]
                    .into_iter()
                    .chain(flags.iter().copied()),
            )
            .unwrap();
            assert_eq!(cli.validate_style_route().is_ok(), engine == "compiler");
        }
    }
    for flags in [
        vec!["--fail-on-new"],
        vec!["--baseline", "old.json", "--write-baseline", "new.json"],
        vec![
            "--write-baseline",
            "new.json",
            "--changed-files",
            "changes.json",
        ],
    ] {
        assert!(
            Cli::try_parse_from(
                ["cargo-rstyle", "--style-engine", "compiler"]
                    .into_iter()
                    .chain(flags)
            )
            .is_err()
        );
    }
    let cli = Cli::try_parse_from([
        "cargo-rstyle",
        "clones",
        "--compiler-advisory",
        "--compiler-baseline",
        "baseline.json",
        "--fail-on-new-compiler",
    ])
    .unwrap();
    assert!(cli.style_baseline.baseline.is_none());
    assert!(!cli.style_baseline.fail_on_new);
    assert!(cli.validate_style_route().is_ok());
}

#[test]
fn root_changed_files_requires_compiler_style_and_is_independent_of_subcommands() {
    assert!(Cli::try_parse_from(["cargo-rstyle", "--changed-files", "changes.json"]).is_err());
    for engine in ["legacy", "compiler"] {
        let cli = Cli::try_parse_from([
            "cargo-rstyle",
            "--style-engine",
            engine,
            "--changed-files",
            "changes.json",
        ])
        .unwrap();
        assert_eq!(cli.validate_style_route().is_ok(), engine == "compiler");
        assert_eq!(
            cli.style_changed_files.as_deref(),
            Some(Path::new("changes.json"))
        );
    }
    let cli = Cli::try_parse_from([
        "cargo-rstyle",
        "clones",
        "--compiler-advisory",
        "--changed-files",
        "changes.json",
    ])
    .unwrap();
    assert!(cli.style_changed_files.is_none());
    assert!(cli.validate_style_route().is_ok());
}

#[test]
fn style_selection_distinguishes_an_absent_flag_from_explicit_backends() {
    let implicit = Cli::try_parse_from(["cargo-rstyle"]).unwrap();
    assert!(implicit.style_engine.is_none());
    for (name, expected) in [("legacy", Engine::Legacy), ("compiler", Engine::Compiler)] {
        let explicit = Cli::try_parse_from(["cargo-rstyle", "--style-engine", name]).unwrap();
        assert!(explicit.style_engine == Some(expected));
    }
}

#[test]
fn subcommands_do_not_inherit_a_style_backend_selection() {
    for command in ["patterns", "clones"] {
        let cli = Cli::try_parse_from(["cargo-rstyle", command]).unwrap();
        assert!(cli.style_engine.is_none());
        assert!(cli.command.is_some());
    }
    let cli = Cli::try_parse_from(["cargo-rstyle", "ignore", "--status"]).unwrap();
    assert!(cli.style_engine.is_none());
    assert!(matches!(cli.command, Some(Command::Ignore(_))));
}

#[test]
fn root_style_replay_is_explicit_and_independent_of_subcommands() {
    let cli = Cli::try_parse_from([
        "cargo-rstyle",
        "--style-engine",
        "compiler",
        "--replay-run",
        "saved-run",
    ])
    .unwrap();
    assert_eq!(
        cli.style_replay_run.as_deref(),
        Some(Path::new("saved-run"))
    );
    assert!(Cli::try_parse_from(["cargo-rstyle", "--replay-run", "saved-run"]).is_err());
    for command in ["patterns", "clones"] {
        let cli =
            Cli::try_parse_from(["cargo-rstyle", command, "--replay-run", "saved-run"]).unwrap();
        assert!(cli.style_replay_run.is_none());
    }
}

#[test]
fn patterns_backend_selection_remains_independent_of_style() {
    for args in [
        vec!["cargo-rstyle", "patterns"],
        vec!["cargo-rstyle", "--style-engine", "compiler", "patterns"],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        let Some(Command::Patterns(patterns)) = cli.command else {
            panic!("expected patterns command");
        };
        assert!(patterns.route.input.engine == Engine::Compiler);
    }
    let cli = Cli::try_parse_from(["cargo-rstyle", "patterns", "--engine", "compiler"]).unwrap();
    assert!(cli.style_engine.is_none());
    let Some(Command::Patterns(patterns)) = cli.command else {
        panic!("expected patterns command");
    };
    assert!(patterns.route.input.engine == Engine::Compiler);
}

#[test]
fn compiler_pattern_dismissal_requires_one_explicit_saved_review_action() {
    let args = [
        "cargo-rstyle",
        "patterns",
        "--engine",
        "compiler",
        "--dismiss-compiler-pattern",
        "0",
        "candidate",
        "--review-file",
        "reviews.json",
        "--review-reason",
        "Reviewed sites",
        "--replay-run",
        "saved-run",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    let Some(Command::Patterns(patterns)) = cli.command else {
        panic!("expected patterns command");
    };
    assert_eq!(patterns.route.dismiss_compiler_pattern, ["0", "candidate"]);
    for missing in ["--review-file", "--review-reason", "--replay-run"] {
        let index = args.iter().position(|arg| *arg == missing).unwrap();
        let incomplete: Vec<_> = args[..index]
            .iter()
            .chain(&args[index + 2..])
            .copied()
            .collect();
        assert!(Cli::try_parse_from(incomplete).is_err(), "{missing}");
    }
    assert!(Cli::try_parse_from(args.iter().copied().chain(["--review-status"])).is_err());
    assert!(
        Cli::try_parse_from([
            "cargo-rstyle",
            "patterns",
            "--engine",
            "compiler",
            "--review-reason",
            "No action"
        ])
        .is_err()
    );
}
