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
        lists: Vec::new(),
    }
}

fn result_to(from: usize, call: usize, position: usize) -> Hop {
    Hop {
        from: Start::Result(from),
        to: End::Argument {
            call,
            position,
            element: None,
        },
    }
}

fn parameter_to(parameter: usize, call: usize, position: usize) -> Hop {
    Hop {
        from: Start::Parameter(parameter),
        to: End::Argument {
            call,
            position,
            element: None,
        },
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

/// The Rust models name these.
const ENV: &str = "std::env::var";
const NEW: &str = "std::process::Command::new";
const ARG: &str = "std::process::Command::arg";
const ARGS: &str = "std::process::Command::args";

fn with_literals(name: &str, line: usize, literals: &[(usize, &str)]) -> FlowCall {
    FlowCall {
        literals: literals
            .iter()
            .map(|(at, text)| (*at, (*text).to_owned()))
            .collect(),
        ..call(name, line)
    }
}

fn in_rust(mut function: FunctionFlows) -> FunctionFlows {
    function.flows.namespace = "rust".into();
    function
}

/// `let input = env::var(..)`, then `Command::new(program)` and an `arg` for each of these in a
/// chain, each on its own line, where `None` is the input.
fn chained(program: &str, arguments: &[Option<&str>]) -> FunctionFlows {
    let mut calls = vec![call(ENV, 1), with_literals(NEW, 2, &[(0, program)])];
    let mut hops = Vec::new();
    for argument in arguments {
        let at = calls.len();
        hops.push(result_to(at - 1, at, 0));
        calls.push(match argument {
            Some(text) => with_literals(ARG, at + 1, &[(1, text)]),
            None => call(ARG, at + 1),
        });
        if argument.is_none() {
            hops.push(result_to(0, at, 1));
        }
    }
    in_rust(function("f", calls, hops))
}

/// The same, as `let mut cmd = Command::new(..); cmd.arg(..);`: a borrow keeps what `cmd` held, so
/// what each call is made on reaches every later one.
fn bound(program: &str, arguments: &[Option<&str>]) -> FunctionFlows {
    let mut bound = chained(program, arguments);
    let made = 1..bound.flows.calls.len();
    for later in made.clone() {
        let earlier = made.clone().filter(|&earlier| earlier + 1 < later);
        let reaching: Vec<Hop> = earlier
            .map(|earlier| result_to(earlier, later, 0))
            .collect();
        bound.flows.hops.extend(reaching);
    }
    bound
}

fn lines(functions: &[FunctionFlows]) -> Vec<usize> {
    found(functions).iter().map(|(_, line, _)| *line).collect()
}

#[test]
fn a_shell_runs_what_a_command_adds_after_its_flag_as_code() {
    let [(holder, line, message)] = &found(&[chained("sh", &[Some("-c"), None])])[..] else {
        panic!("one finding");
    };
    assert_eq!((*holder, *line), (0, 4), "the arg it's given to");
    assert!(
        message.contains("`std::process::Command::arg`"),
        "{message}"
    );
    assert!(message.contains("command injection, CWE-78"), "{message}");
    assert_eq!(lines(&[chained("/bin/bash", &[Some("-c"), None])]), [4]);

    for (program, arguments, why) in [
        ("ls", &[None][..], "argv"),
        ("ls", &[Some("-c"), None], "not a shell"),
        ("sh", &[None], "a script's path"),
        ("sh", &[Some("script.sh"), None], "data to the script"),
        ("sh", &[Some("-x"), None], "not told to"),
        (
            "sh",
            &[Some("-c"), Some("echo $0"), None],
            "data to the code",
        ),
    ] {
        assert!(lines(&[chained(program, arguments)]).is_empty(), "{why}");
    }
}

#[test]
fn a_command_bound_to_a_name_is_read_in_the_order_its_calls_ran() {
    assert_eq!(lines(&[bound("sh", &[Some("-c"), None])]), [4]);
    assert!(
        lines(&[bound("sh", &[Some("-c"), Some("echo $0"), None])]).is_empty(),
        "what -c added still reaches the last arg, but the one after it came between"
    );
    assert!(lines(&[bound("sh", &[Some("script.sh"), None])]).is_empty());
}

/// `Command::new("sh").args(list)` and then `.arg(..)`, where `list` is `None` when it isn't written
/// out. The input is that element of the list, or what `arg` is given when there's none.
fn listed(list: Option<&[Option<&str>]>, input: Option<usize>) -> FunctionFlows {
    let written_out = list.map(|list| (1, list.iter().map(|it| it.map(str::to_owned)).collect()));
    let args = FlowCall {
        lists: written_out.into_iter().collect(),
        ..call(ARGS, 3)
    };
    let calls = vec![
        call(ENV, 1),
        with_literals(NEW, 2, &[(0, "sh")]),
        args,
        call(ARG, 4),
    ];
    let to = match input {
        Some(element) => End::Argument {
            call: 2,
            position: 1,
            element: Some(element),
        },
        None => result_to(0, 3, 1).to,
    };
    let from = Start::Result(0);
    let hops = vec![result_to(1, 2, 0), result_to(2, 3, 0), Hop { from, to }];
    in_rust(function("f", calls, hops))
}

#[test]
fn each_element_of_a_list_written_out_is_an_argument_of_its_own() {
    let code = [Some("-c"), None, None];
    assert_eq!(lines(&[listed(Some(&code), Some(1))]), [3]);
    assert!(
        lines(&[listed(Some(&code), Some(2))]).is_empty(),
        "data to the code"
    );
    assert_eq!(
        lines(&[listed(Some(&[Some("-c")]), None)]),
        [4],
        "the list counts"
    );
    assert!(
        lines(&[listed(None, None)]).is_empty(),
        "a list that isn't written out can't be counted"
    );
}

/// `Command::new("sh").between().arg("-c").arg(input)`
fn with_a_call_between(between: &str) -> FunctionFlows {
    let calls = vec![
        call(ENV, 1),
        with_literals(NEW, 2, &[(0, "sh")]),
        call(between, 2),
        with_literals(ARG, 3, &[(1, "-c")]),
        call(ARG, 4),
    ];
    let made_on = (1..4).map(|on| result_to(on, on + 1, 0));
    let hops = made_on.chain([result_to(0, 4, 1)]).collect();
    in_rust(function("f", calls, hops))
}

#[test]
fn a_call_between_with_no_body_here_keeps_the_command() {
    let between = with_a_call_between("std::process::Command::current_dir");
    assert_eq!(lines(&[between]), [4]);

    let configure = function(
        "configure",
        Vec::new(),
        vec![returned_from(Start::Parameter(0))],
    );
    assert!(
        lines(&[with_a_call_between("configure"), configure]).is_empty(),
        "a body here could add an argument"
    );
}

#[test]
fn a_helper_that_runs_its_parameter_in_a_shell_is_where_it_is_reported() {
    let run = in_rust(function(
        "run",
        vec![
            with_literals(NEW, 2, &[(0, "sh")]),
            with_literals(ARG, 3, &[(1, "-c")]),
            call(ARG, 4),
        ],
        vec![
            result_to(0, 1, 0),
            result_to(1, 2, 0),
            parameter_to(0, 2, 1),
        ],
    ));
    let caller = in_rust(function(
        "caller",
        vec![call(ENV, 7), call("run", 8)],
        vec![result_to(0, 1, 0)],
    ));
    let [(holder, line, message)] = &found(&[run, caller])[..] else {
        panic!("one finding");
    };
    assert_eq!((*holder, *line), (0, 4));
    assert!(message.contains("caller.go:7"), "{message}");
}

fn ignoring(key: &str) -> TaintConfig {
    TaintConfig {
        ignored: vec![key.into()],
        ..TaintConfig::default()
    }
}

/// `Command::new("sh").arg("-c").arg(input)`, chained and through `let mut cmd`.
fn shells() -> [(&'static str, FunctionFlows); 2] {
    let code = [Some("-c"), None];
    [
        ("chained", chained("sh", &code)),
        ("bound", bound("sh", &code)),
    ]
}

#[test]
fn a_project_can_say_what_a_shell_is_given_doesnt_count() {
    for (form, shell) in shells() {
        let found = |key: &str| found_with(std::slice::from_ref(&shell), &ignoring(key)).len();
        assert_eq!(found(ARG), 0, "{form}: where it's added");
        assert_eq!(found(NEW), 0, "{form}: what made the command");
        for other in [
            "std::process::Command::current_dir",
            "std::process::Command",
            "os/exec.Command",
        ] {
            assert_eq!(found(other), 1, "{form}: {other}");
        }
    }
}

#[test]
fn what_made_a_command_is_ignored_by_any_name_it_has() {
    for (form, mut shell) in shells() {
        let new = &mut shell.flows.calls[1];
        new.identity = "std::process::{impl#20}::new".into();
        new.operation = Some("rust:Command.new".into());
        let shell = std::slice::from_ref(&shell);
        assert_eq!(found(shell).len(), 1, "{form}");
        for key in [
            NEW,
            "std::process::{impl#20}::new",
            "rust:Command.new",
            "Command.new",
        ] {
            assert!(
                found_with(shell, &ignoring(key)).is_empty(),
                "{form}: {key}"
            );
        }
    }
}

#[test]
fn ignoring_what_made_a_command_leaves_other_ends_of_the_input() {
    for (form, mut shell) in shells() {
        let read = shell.flows.calls.len();
        shell.flows.calls.push(call("std::fs::read", 9));
        shell.flows.hops.push(result_to(0, read, 0));
        let shell = std::slice::from_ref(&shell);
        assert_eq!(lines(shell), [4, 9], "{form}");
        let found = found_with(shell, &ignoring(NEW));
        assert_eq!(found.len(), 1, "{form}: {found:?}");
        assert!(found[0].2.contains("path traversal"), "{form}: {found:?}");
    }

    let program = in_rust(function(
        "f",
        vec![call(ENV, 1), call(NEW, 2)],
        vec![result_to(0, 1, 0)],
    ));
    let program = std::slice::from_ref(&program);
    assert_eq!(found_with(program, &ignoring(ARG)).len(), 1);
    assert!(
        found_with(program, &ignoring(NEW)).is_empty(),
        "the program it's given is the same sink"
    );
}

#[test]
fn a_go_command_is_one_call_so_ignoring_it_ignores_what_its_shell_runs() {
    let shell = function(
        "f",
        vec![call(SOURCE, 1), command(&[(0, "sh"), (1, "-c")])],
        vec![result_to(0, 1, 2)],
    );
    let shell = std::slice::from_ref(&shell);
    assert_eq!(found(shell).len(), 1);
    assert!(found_with(shell, &ignoring("os/exec.Command")).is_empty());
}

#[test]
fn commands_built_around_a_loop_settle() {
    let mut looped = chained("sh", &[Some("-v"), None]);
    looped.flows.hops.push(result_to(3, 2, 0));
    assert!(lines(&[looped]).is_empty());
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
    for namespace in ["go", "cxx", "rust"] {
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
