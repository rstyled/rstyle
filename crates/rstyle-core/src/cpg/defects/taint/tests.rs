use super::super::value_flow::Hop;
use super::*;
use crate::config::TaintSink;

/// The Go models name these, so the tests read as what they're about.
const SOURCE: &str = "os.Getenv";
const SINK: &str = "os.Open";
const QUERY: &str = "database/sql.DB.Exec";

fn call(name: &str, line: usize) -> FlowCall {
    FlowCall {
        identity: name.into(),
        selected: name.into(),
        name: name.into(),
        operation: None,
        line,
        literals: Vec::new(),
        keywords: Vec::new(),
    }
}

fn result_to(from: usize, call: usize, position: usize) -> Hop {
    Hop {
        from: Start::Result(from),
        to: End::Argument { call, position },
    }
}

fn parameter_to(parameter: usize, call: usize, position: usize) -> Hop {
    Hop {
        from: Start::Parameter(parameter),
        to: End::Argument { call, position },
    }
}

fn returned_from(from: Start) -> Hop {
    Hop {
        from,
        to: End::Returned,
    }
}

fn function(identity: &str, calls: Vec<FlowCall>, hops: Vec<Hop>) -> FunctionFlows {
    FunctionFlows {
        identity: identity.into(),
        path: format!("{identity}.go"),
        allowed: false,
        flows: ValueFlows {
            identity: identity.into(),
            namespace: "go".into(),
            calls,
            hops,
        },
    }
}

/// One function as a Python frontend writes it, read by Python's models.
fn in_python(calls: Vec<FlowCall>, hops: Vec<Hop>) -> FunctionFlows {
    let mut function = function("f", calls, hops);
    function.flows.namespace = "py".into();
    function
}

fn found_with(functions: &[FunctionFlows], project: &TaintConfig) -> Vec<(usize, usize, String)> {
    let found = find(functions, project).into_iter();
    found
        .map(|it| (it.function, it.defect.line, it.defect.message))
        .collect()
}

fn found(functions: &[FunctionFlows]) -> Vec<(usize, usize, String)> {
    found_with(functions, &TaintConfig::default())
}

/// `sink(source())`, each call on its own line.
fn source_into_sink() -> FunctionFlows {
    function(
        "f",
        vec![call(SOURCE, 1), call(SINK, 2)],
        vec![result_to(0, 1, 0)],
    )
}

#[test]
fn input_reaching_a_dangerous_argument_says_where_from_and_what_it_risks() {
    let [(holder, line, message)] = &found(&[source_into_sink()])[..] else {
        panic!("one finding");
    };
    assert_eq!((*holder, *line), (0, 2));
    assert!(message.contains("`os.Getenv` on line 1"), "{message}");
    assert!(message.contains("`os.Open`"), "{message}");
    assert!(message.contains("path traversal, CWE-22"), "{message}");
}

#[test]
fn an_argument_a_sink_takes_as_data_is_not_dangerous() {
    let parameterized = function(
        "f",
        vec![call(SOURCE, 1), call(QUERY, 2)],
        vec![result_to(0, 1, 2)],
    );
    assert!(found(&[parameterized]).is_empty());

    let the_query = function(
        "f",
        vec![call(SOURCE, 1), call(QUERY, 2)],
        vec![result_to(0, 1, 1)],
    );
    assert_eq!(found(&[the_query]).len(), 1);
}

#[test]
fn a_call_that_is_neither_end_has_nothing_to_say() {
    let neither = function(
        "f",
        vec![call("a", 1), call("b", 2)],
        vec![result_to(0, 1, 0)],
    );
    assert!(found(&[neither]).is_empty());
}

fn command(literals: &[(usize, &str)]) -> FlowCall {
    FlowCall {
        literals: literals
            .iter()
            .map(|(at, text)| (*at, (*text).to_owned()))
            .collect(),
        ..call("os/exec.Command", 2)
    }
}

fn into_command(literals: &[(usize, &str)], position: usize) -> Vec<(usize, usize, String)> {
    found(&[function(
        "f",
        vec![call(SOURCE, 1), command(literals)],
        vec![result_to(0, 1, position)],
    )])
}

#[test]
fn a_programs_arguments_are_data_unless_a_shell_is_told_to_run_one() {
    assert_eq!(into_command(&[], 0).len(), 1, "the program itself");
    assert!(into_command(&[(0, "ls")], 1).is_empty(), "argv");
    assert!(
        into_command(&[(0, "ls"), (1, "-c")], 2).is_empty(),
        "not a shell"
    );
    assert!(
        into_command(&[(0, "sh"), (1, "-x")], 2).is_empty(),
        "not told to"
    );
    assert!(into_command(&[], 2).is_empty(), "nothing says it's a shell");

    assert_eq!(into_command(&[(0, "sh"), (1, "-c")], 2).len(), 1);
    assert_eq!(into_command(&[(0, "/bin/bash"), (1, "-c")], 2).len(), 1);
    assert!(
        into_command(&[(0, "sh"), (1, "-c")], 3).is_empty(),
        "data to the script"
    );
}

/// `subprocess.run(os.getenv(..), ..)` with these keywords, and input at this position.
fn into_run(keywords: &[(&str, &str)], position: usize) -> Vec<(usize, usize, String)> {
    let run = FlowCall {
        keywords: keywords
            .iter()
            .map(|(name, literal)| ((*name).to_owned(), (*literal).to_owned()))
            .collect(),
        ..call("subprocess.run", 2)
    };
    let calls = vec![call("os.getenv", 1), run];
    found(&[in_python(calls, vec![result_to(0, 1, position)])])
}

#[test]
fn a_sink_that_needs_a_keyword_is_one_only_where_the_call_passes_its_literal() {
    assert_eq!(into_run(&[("shell", "true")], 0).len(), 1);
    assert!(into_run(&[], 0).is_empty(), "no shell");
    assert!(into_run(&[("shell", "false")], 0).is_empty());
    assert!(
        into_run(&[("check", "true")], 0).is_empty(),
        "another keyword"
    );
    assert!(
        into_run(&[("shell", "true")], 1).is_empty(),
        "not the command"
    );
}

/// `sink(through(source()))`
fn through(name: &str) -> FunctionFlows {
    function(
        "f",
        vec![call(SOURCE, 1), call(name, 2), call(SINK, 3)],
        vec![result_to(0, 1, 0), result_to(1, 2, 0)],
    )
}

#[test]
fn a_call_with_no_body_here_gives_back_what_it_was_handed() {
    assert_eq!(found(&[through("library.Format")]).len(), 1);
}

#[test]
fn a_project_says_what_cleans_input() {
    let project = TaintConfig {
        sanitizers: vec!["library.Escape".into()],
        ..TaintConfig::default()
    };
    assert!(found_with(&[through("library.Escape")], &project).is_empty());
    assert_eq!(found_with(&[through("library.Format")], &project).len(), 1);
}

#[test]
fn a_model_says_what_cleans_input_and_a_project_can_take_that_back() {
    let quoted = || {
        let calls = vec![
            call("os.getenv", 1),
            call("shlex.quote", 2),
            call("os.system", 3),
        ];
        in_python(calls, vec![result_to(0, 1, 0), result_to(1, 2, 0)])
    };
    assert!(found(&[quoted()]).is_empty());

    let project = TaintConfig {
        ignored: vec!["shlex.quote".into()],
        ..TaintConfig::default()
    };
    assert_eq!(found_with(&[quoted()], &project).len(), 1);
}

#[test]
fn a_helpers_body_says_which_arguments_it_gives_back() {
    let passes = function("pass", Vec::new(), vec![returned_from(Start::Parameter(0))]);
    assert_eq!(found(&[through("pass"), passes]).len(), 1);

    let ignores = function("pass", Vec::new(), Vec::new());
    assert!(found(&[through("pass"), ignores]).is_empty());
}

#[test]
fn a_helper_can_return_input_of_its_own() {
    let input = function(
        "input",
        vec![call(SOURCE, 7)],
        vec![returned_from(Start::Result(0))],
    );
    let caller = function(
        "caller",
        vec![call("input", 1), call(SINK, 2)],
        vec![result_to(0, 1, 0)],
    );
    let [(holder, line, message)] = &found(&[input, caller])[..] else {
        panic!("one finding");
    };
    assert_eq!((*holder, *line), (1, 2));
    assert!(message.contains("`os.Getenv` on input.go:7"), "{message}");
}

#[test]
fn a_helper_can_hold_the_sink_and_that_is_where_it_is_reported() {
    let output = function("output", vec![call(SINK, 9)], vec![parameter_to(0, 0, 0)]);
    let caller = function(
        "caller",
        vec![call(SOURCE, 1), call("output", 2)],
        vec![result_to(0, 1, 0)],
    );
    let [(holder, line, message)] = &found(&[output, caller])[..] else {
        panic!("one finding");
    };
    assert_eq!((*holder, *line), (0, 9));
    assert!(message.contains("caller.go:1"), "{message}");

    let other_argument = function(
        "caller",
        vec![call(SOURCE, 1), call("output", 2)],
        vec![result_to(0, 1, 1)],
    );
    let output = function("output", vec![call(SINK, 9)], vec![parameter_to(0, 0, 0)]);
    assert!(found(&[output, other_argument]).is_empty());
}

#[test]
fn a_value_is_followed_through_any_number_of_helpers() {
    let last = function("last", vec![call(SINK, 9)], vec![parameter_to(0, 0, 0)]);
    let middle = function("middle", vec![call("last", 5)], vec![parameter_to(0, 0, 0)]);
    let caller = function(
        "caller",
        vec![call(SOURCE, 1), call("middle", 2)],
        vec![result_to(0, 1, 0)],
    );
    let found = found(&[last, middle, caller]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (0, 9));
}

#[test]
fn functions_that_call_each_other_settle() {
    let a = function(
        "a",
        vec![call("b", 1), call(SINK, 2)],
        vec![parameter_to(0, 0, 0), parameter_to(0, 1, 0)],
    );
    let b = function("b", vec![call("a", 3)], vec![parameter_to(0, 0, 0)]);
    let caller = function(
        "caller",
        vec![call(SOURCE, 4), call("b", 5)],
        vec![result_to(0, 1, 0)],
    );
    assert_eq!(found(&[a, b, caller]).len(), 1);
}

#[test]
fn the_body_that_runs_is_the_implementation_a_call_selects() {
    let declared = FlowCall {
        selected: "Impl.output".into(),
        ..call("Trait.output", 2)
    };
    let caller = function(
        "caller",
        vec![call(SOURCE, 1), declared],
        vec![result_to(0, 1, 0)],
    );
    let implementation = function(
        "Impl.output",
        vec![call(SINK, 9)],
        vec![parameter_to(0, 0, 0)],
    );
    assert_eq!(found(&[caller, implementation]).len(), 1);
}

#[test]
fn a_project_adds_its_own_ends_by_any_name_a_call_has() {
    let flows = || {
        let source = FlowCall {
            operation: Some("go:my/pkg.Read".into()),
            ..call("(*my/pkg.R).Read", 1)
        };
        let sink = FlowCall {
            identity: "go::my/pkg.Run".into(),
            ..call("my/pkg.Run", 2)
        };
        function("f", vec![source, sink], vec![result_to(0, 1, 1)])
    };
    assert!(found(&[flows()]).is_empty(), "nothing names them yet");

    let by = |source: &str, sink: &str, position: usize| TaintConfig {
        sources: vec![source.into()],
        sinks: vec![TaintSink {
            callees: vec![sink.into()],
            arguments: vec![position],
            risk: "running queries".into(),
            cwe: None,
        }],
        ..TaintConfig::default()
    };
    for (source, sink) in [
        ("my/pkg.Read", "my/pkg.Run"),
        ("go:my/pkg.Read", "go::my/pkg.Run"),
        ("(*my/pkg.R).Read", "my/pkg.Run"),
    ] {
        let found = found_with(&[flows()], &by(source, sink, 1));
        assert_eq!(found.len(), 1, "{source} {sink}");
        assert!(found[0].2.contains("(running queries)"), "{}", found[0].2);
    }
    assert!(found_with(&[flows()], &by("my/pkg.Read", "my/pkg.Run", 0)).is_empty());
}

#[test]
fn a_project_says_what_doesnt_count_for_it() {
    let project = |ignored: &str| TaintConfig {
        ignored: vec![ignored.into()],
        ..TaintConfig::default()
    };
    assert!(found_with(&[source_into_sink()], &project(SOURCE)).is_empty());
    assert!(found_with(&[source_into_sink()], &project(SINK)).is_empty());
    assert_eq!(
        found_with(&[source_into_sink()], &project("other")).len(),
        1
    );
}

#[test]
fn a_helper_returning_ignored_input_returns_nothing() {
    let input = || {
        function(
            "input",
            vec![call(SOURCE, 7)],
            vec![returned_from(Start::Result(0))],
        )
    };
    let caller = || {
        function(
            "caller",
            vec![call("input", 1), call(SINK, 2)],
            vec![result_to(0, 1, 0)],
        )
    };
    assert_eq!(found(&[input(), caller()]).len(), 1);
    let project = TaintConfig {
        ignored: vec![SOURCE.into()],
        ..TaintConfig::default()
    };
    assert!(found_with(&[input(), caller()], &project).is_empty());
}

#[test]
fn every_model_file_is_read_and_a_namespace_without_one_names_nothing() {
    for namespace in ["go", "cxx", "rust", "py"] {
        let models = Models::of(namespace);
        assert!(!models.sources.is_empty(), "{namespace}");
        assert!(!models.sinks.is_empty(), "{namespace}");
    }
    assert!(Models::of("cobol").sources.is_empty());
}

#[test]
fn a_call_is_named_by_its_operation_its_name_or_its_identity() {
    let called = FlowCall {
        identity: "go::(*os.File).Read".into(),
        operation: Some("go:os.File.Read".into()),
        ..call("(*os.File).Read", 1)
    };
    for key in [
        "os.File.Read",
        "go:os.File.Read",
        "(*os.File).Read",
        "go::(*os.File).Read",
    ] {
        assert!(names(key, &called), "{key}");
    }
    assert!(!names("os.File", &called), "a whole name, not a prefix");
}

#[test]
fn what_cleans_input_cant_also_return_it() {
    let project = TaintConfig {
        sanitizers: vec!["input".into()],
        ..TaintConfig::default()
    };
    let input = function(
        "input",
        vec![call(SOURCE, 7)],
        vec![returned_from(Start::Result(0))],
    );
    let caller = function(
        "caller",
        vec![call("input", 1), call(SINK, 2)],
        vec![result_to(0, 1, 0)],
    );
    assert!(found_with(&[input, caller], &project).is_empty());
}

#[test]
fn rstyle_toml_adds_to_what_taint_reads() {
    use crate::config::Config;

    let taint = Config::parse("").unwrap().taint;
    assert!(taint.sources.is_empty() && taint.sinks.is_empty());
    assert!(taint.sanitizers.is_empty() && taint.ignored.is_empty());

    let taint = Config::parse(
        "[taint]\nsources = [\"a\"]\nsanitizers = [\"b\"]\nignored = [\"c\"]\n\
         [[taint.sinks]]\ncallees = [\"d\"]\narguments = [1]\nrisk = \"SQL injection\"\n",
    )
    .unwrap()
    .taint;
    assert_eq!(taint.sources, ["a"]);
    assert_eq!(taint.sanitizers, ["b"]);
    assert_eq!(taint.ignored, ["c"]);
    assert_eq!(taint.sinks[0].callees, ["d"]);
    assert_eq!(taint.sinks[0].arguments, [1]);
    assert_eq!(taint.sinks[0].cwe, None);

    assert!(Config::parse("[taint]\nsource = [\"a\"]").is_err());
    let no_risk = "[[taint.sinks]]\ncallees = [\"d\"]\narguments = [1]";
    assert!(Config::parse(no_risk).is_err());
}

#[test]
fn a_baseline_does_not_compare_what_taint_reads() {
    use crate::config::Config;

    let policy = |input: &str| serde_json::to_value(Config::parse(input).unwrap()).unwrap();
    assert_eq!(policy("[taint]\nsources = [\"a\"]"), policy(""));
}
