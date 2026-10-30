use super::*;

#[test]
fn removed_engine_selectors_are_rejected_before_capture() {
    for args in [
        vec!["--style-engine", "legacy"],
        vec!["--style-engine", "compiler"],
        vec!["patterns", "--engine", "legacy"],
        vec!["patterns", "--engine", "compiler"],
        vec!["ignore", "--engine", "legacy", "--status"],
    ] {
        assert!(Cli::try_parse_from(["cargo-rstyle"].into_iter().chain(args)).is_err());
    }
}

#[test]
fn root_style_baselines_require_explicit_gate_inputs() {
    for flags in [
        vec!["--baseline", "baseline.json"],
        vec!["--write-baseline", "baseline.json"],
        vec!["--baseline", "baseline.json", "--fail-on-new"],
    ] {
        let cli = Cli::try_parse_from(["cargo-rstyle"].into_iter().chain(flags)).unwrap();
        assert!(cli.validate_style_route().is_ok());
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
        assert!(Cli::try_parse_from(["cargo-rstyle"].into_iter().chain(flags)).is_err());
    }
}

#[test]
fn root_inputs_remain_independent_of_subcommands() {
    let root = Cli::try_parse_from([
        "cargo-rstyle",
        "--replay-run",
        "saved",
        "--changed-files",
        "changes.json",
    ])
    .unwrap();
    assert_eq!(
        root.style_inputs.replay_run.as_deref(),
        Some(Path::new("saved"))
    );
    assert_eq!(
        root.style_inputs.changed_files.as_deref(),
        Some(Path::new("changes.json"))
    );
    assert!(root.validate_style_route().is_ok());
    for command in ["patterns", "clones"] {
        let cli = Cli::try_parse_from(["cargo-rstyle", command, "--replay-run", "saved"]).unwrap();
        assert!(!cli.style_inputs.requested());
        assert!(cli.validate_style_route().is_ok());
        for option in ["--replay-run", "--changed-files"] {
            let cli = Cli::try_parse_from(["cargo-rstyle", option, "saved", command]).unwrap();
            assert!(cli.validate_style_route().is_err());
        }
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
    assert!(!cli.style_inputs.requested());
}

#[test]
fn compiler_pattern_dismissal_requires_one_saved_review_action() {
    let args = [
        "cargo-rstyle",
        "patterns",
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
        panic!("patterns command")
    };
    assert_eq!(patterns.route.dismiss_compiler_pattern, ["0", "candidate"]);
    for missing in ["--review-file", "--review-reason", "--replay-run"] {
        let i = args.iter().position(|arg| *arg == missing).unwrap();
        assert!(Cli::try_parse_from(args[..i].iter().chain(&args[i + 2..]).copied()).is_err());
    }
    assert!(Cli::try_parse_from(args.iter().copied().chain(["--review-status"])).is_err());
}

#[test]
fn style_reviews_reuse_explicit_saved_finding_actions() {
    let args = [
        "cargo-rstyle",
        "--review-file",
        "reviews.json",
        "--replay-run",
        "saved-run",
        "--dismiss-compiler-diagnostic",
        "0",
        "1",
        "--review-reason",
        "Reviewed IO boundary",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(cli.validate_style_route().is_ok());
    assert_eq!(
        cli.style_inputs.review.dismiss_compiler_diagnostic,
        ["0", "1"]
    );
    for missing in ["--review-file", "--review-reason", "--replay-run"] {
        let i = args.iter().position(|arg| *arg == missing).unwrap();
        assert!(Cli::try_parse_from(args[..i].iter().chain(&args[i + 2..]).copied()).is_err());
    }
    assert!(Cli::try_parse_from(args.iter().copied().chain(["--review-status"])).is_err());
}
