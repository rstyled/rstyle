use super::*;

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
