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
fn patterns_backend_selection_remains_independent_of_style() {
    for args in [
        vec!["cargo-rstyle", "patterns"],
        vec!["cargo-rstyle", "--style-engine", "compiler", "patterns"],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        let Some(Command::Patterns(patterns)) = cli.command else {
            panic!("expected patterns command");
        };
        assert!(patterns.route.engine == Engine::Legacy);
    }
    let cli = Cli::try_parse_from(["cargo-rstyle", "patterns", "--engine", "compiler"]).unwrap();
    assert!(cli.style_engine.is_none());
    let Some(Command::Patterns(patterns)) = cli.command else {
        panic!("expected patterns command");
    };
    assert!(patterns.route.engine == Engine::Compiler);
}
