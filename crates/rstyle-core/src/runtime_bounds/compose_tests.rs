use super::compose::{Analyzed, Composed, Composer};
use super::*;
use std::collections::BTreeMap;

fn node(action: Action, flow: Flow) -> Node {
    Node {
        action,
        flow,
        source: None,
    }
}

fn call(target: &str, arguments: Vec<Argument>) -> Action {
    Action::Call {
        callee: target.into(),
        target: match target {
            "ctor" => Callee::Constructor,
            "?" => Callee::Unresolved,
            library if library.starts_with("std::") => Callee::Library(library.into()),
            _ => Callee::Resolved(target.into()),
        },
        arguments,
        may_modify: false,
    }
}

fn field(index: u32, name: &str) -> Field {
    Field {
        index,
        name: name.into(),
    }
}

/// One owner: `for i in 0..limit { calls }` when `limit` is set, else straight-line calls.
/// Variable `inputs.len()` is the loop counter `i`.
struct Shape {
    inputs: Vec<&'static str>,
    formals: Vec<Formal>,
    extents: Vec<Extent>,
    limit: Option<&'static str>,
    calls: Vec<Action>,
}

impl Shape {
    fn new(inputs: &[&'static str]) -> Self {
        Self {
            inputs: inputs.to_vec(),
            formals: Vec::new(),
            extents: Vec::new(),
            limit: None,
            calls: Vec::new(),
        }
    }

    fn scalars(mut self) -> Self {
        self.formals = self
            .inputs
            .iter()
            .map(|name| Formal {
                name: (*name).into(),
                measure: Measure::Scalar {
                    input: (*name).into(),
                },
            })
            .collect();
        self
    }

    fn formals(mut self, formals: Vec<Formal>) -> Self {
        self.formals = formals;
        self
    }

    fn extent(mut self, input: &str, root: Root, fields: Vec<Field>) -> Self {
        self.extents.push(Extent {
            input: input.into(),
            root,
            fields,
        });
        self
    }

    fn scan(mut self, limit: &'static str) -> Self {
        self.limit = Some(limit);
        self
    }

    fn calls(mut self, calls: Vec<Action>) -> Self {
        self.calls = calls;
        self
    }

    fn counter(&self) -> u32 {
        self.inputs.len() as u32
    }

    fn program(self) -> Program {
        let variables = self.variables();
        let (mut nodes, loop_headers) = self.prologue();
        let first_call = nodes.len() as u32;
        let looping = self.limit.is_some();
        for (offset, action) in self.calls.into_iter().enumerate() {
            nodes.push(node(action, Flow::Goto(first_call + offset as u32 + 1)));
        }
        if looping {
            let variable = self.inputs.len() as u32;
            nodes.push(node(Action::Increment { variable }, Flow::Goto(1)));
        }
        nodes.push(node(Action::Nop, Flow::Return));

        Program {
            inputs: self.inputs.iter().map(|name| (*name).into()).collect(),
            variables,
            nodes,
            entry: 0,
            loop_headers,
            arithmetic: Arithmetic::Checked,
            formals: self.formals,
            extents: self.extents,
        }
    }

    fn variables(&self) -> Vec<Variable> {
        let inputs = self.inputs.iter().map(|name| Variable {
            name: (*name).into(),
            initial: Value::Input((*name).into()),
            bits: 64,
        });
        let counter = Variable {
            name: "i".into(),
            initial: Value::Unknown,
            bits: 64,
        };
        inputs.chain(std::iter::once(counter)).collect()
    }

    // `i = 0; while i < limit` guard nodes, exiting past the calls and increment.
    fn prologue(&self) -> (Vec<Node>, Vec<u32>) {
        let Some(limit) = self.limit else {
            return (Vec::new(), Vec::new());
        };
        let counter = self.counter();
        let limit = self.inputs.iter().position(|n| *n == limit).unwrap() as u32;
        let exit = 2 + self.calls.len() as u32 + 1;
        let start = Action::Assign {
            variable: counter,
            value: Value::Const(0),
        };
        let guard = Flow::Branch {
            condition: Condition::Less {
                variable: counter,
                limit: Value::Variable(limit),
            },
            then_node: 2,
            else_node: exit,
        };
        let nodes = vec![node(start, Flow::Goto(1)), node(Action::Nop, guard)];
        (nodes, vec![1])
    }
}

fn compose(owners: Vec<(&str, Program)>, root: &str) -> Composed {
    compose_with(owners, root, Limits::default())
}

fn compose_with(owners: Vec<(&str, Program)>, root: &str, limits: Limits) -> Composed {
    let owners: BTreeMap<String, Program> = owners
        .into_iter()
        .map(|(id, program)| (id.to_string(), program))
        .collect();
    let mut composer = Composer::new(
        |id: &str| {
            owners.get(id).map(|program| Analyzed {
                outcome: analyze(program),
                program: program.clone(),
            })
        },
        limits,
    );
    composer.composed(root)
}

fn proven(composed: Composed) -> compose::Composition {
    match composed {
        Composed::Proven(composition) => composition,
        other => panic!("{other:?}"),
    }
}

fn unknown(composed: Composed) -> String {
    match composed {
        Composed::Unknown { reason } => reason,
        other => panic!("{other:?}"),
    }
}

fn eval(bound: &Bound, values: &[(&str, u64)]) -> u64 {
    match bound {
        Bound::Const(value) => *value,
        Bound::Input(name) => {
            values
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("unbound input {name}"))
                .1
        }
        Bound::Add(items) => items.iter().map(|item| eval(item, values)).sum(),
        Bound::Mul(items) => items.iter().map(|item| eval(item, values)).product(),
        Bound::Max(items) => items
            .iter()
            .map(|item| eval(item, values))
            .max()
            .unwrap_or(0),
    }
}

fn linear() -> Program {
    Shape::new(&["n"]).scalars().scan("n").program()
}

#[test]
fn independent_caller_sizes_substitute_into_callee_summaries() {
    let caller = Shape::new(&["m", "q"])
        .scalars()
        .scan("m")
        .calls(vec![call(
            "w::linear",
            vec![Argument::Scalar(Value::Input("q".into()))],
        )])
        .program();
    let result = proven(compose(
        vec![("w::linear", linear()), ("w::caller", caller)],
        "w::caller",
    ));
    // m outer entries + m invocations * q callee entries.
    assert_eq!(
        eval(&result.loop_body_entries, &[("m", 4), ("q", 7)]),
        4 + 4 * 7
    );
    assert_eq!(
        eval(&result.workspace_invocations, &[("m", 4), ("q", 7)]),
        4
    );
    assert_eq!(result.resolved_callees, vec!["w::linear".to_string()]);
}

#[test]
fn per_item_varying_sizes_use_the_guarded_counter_maximum() {
    let caller = Shape::new(&["m"]).scalars().scan("m");
    let counter = caller.counter();
    let caller = caller
        .calls(vec![call(
            "w::linear",
            vec![Argument::Scalar(Value::Variable(counter))],
        )])
        .program();
    let result = proven(compose(
        vec![("w::linear", linear()), ("w::caller", caller)],
        "w::caller",
    ));
    // Actual work is sum(i for i < m) = m(m-1)/2; the monotone maximum bound is m + m*m.
    for m in 0..8u64 {
        let bound = eval(&result.loop_body_entries, &[("m", m)]);
        assert!(m + m * m.saturating_sub(1) / 2 <= bound);
        assert_eq!(bound, m + m * m);
    }
}

#[test]
fn length_inputs_follow_shared_places_and_field_paths() {
    let callee = Shape::new(&["len(xs)#2"])
        .formals(vec![Formal {
            name: "xs".into(),
            measure: Measure::Shared,
        }])
        .extent("len(xs)#2", Root::Formal(0), Vec::new())
        .scan("len(xs)#2")
        .program();
    let place = Argument::Place {
        root: Root::Formal(0),
        fields: vec![field(1, "items")],
    };
    let shared = vec![Formal {
        name: "graph".into(),
        measure: Measure::Shared,
    }];
    let synthesized = Shape::new(&[])
        .formals(shared.clone())
        .calls(vec![call("w::walk", vec![place.clone()])])
        .program();
    let result = proven(compose(
        vec![("w::walk", callee.clone()), ("w::caller", synthesized)],
        "w::caller",
    ));
    assert_eq!(
        result.loop_body_entries,
        Bound::Input("len(graph.items)".into())
    );

    let existing = Shape::new(&["len(graph.items)#7"])
        .formals(shared)
        .extent(
            "len(graph.items)#7",
            Root::Formal(0),
            vec![field(1, "items")],
        )
        .calls(vec![call("w::walk", vec![place])])
        .program();
    let result = proven(compose(
        vec![("w::walk", callee), ("w::caller", existing)],
        "w::caller",
    ));
    assert_eq!(
        result.loop_body_entries,
        Bound::Input("len(graph.items)#7".into())
    );
}

#[test]
fn synthesized_lengths_compose_through_intermediate_callers() {
    let leaf = Shape::new(&["len(p)#1"])
        .formals(vec![Formal {
            name: "p".into(),
            measure: Measure::Shared,
        }])
        .extent("len(p)#1", Root::Formal(0), Vec::new())
        .scan("len(p)#1")
        .program();
    let forward = |name: &str, target: &str, field_name: &str| {
        Shape::new(&[])
            .formals(vec![Formal {
                name: name.into(),
                measure: Measure::Shared,
            }])
            .calls(vec![
                call(
                    target,
                    vec![Argument::Place {
                        root: Root::Formal(0),
                        fields: vec![field(0, field_name)],
                    }],
                ),
                call("ctor", Vec::new()),
                call("std::vec::Vec::push", Vec::new()),
            ])
            .program()
    };
    let result = proven(compose(
        vec![
            ("w::leaf", leaf),
            ("w::mid", forward("g", "w::leaf", "items")),
            ("w::top", forward("t", "w::mid", "inner")),
        ],
        "w::top",
    ));
    assert_eq!(
        result.loop_body_entries,
        Bound::Input("len(t.inner.items)".into())
    );
    assert_eq!(eval(&result.workspace_invocations, &[]), 2);
    assert_eq!(
        result.excluded_library_callees,
        vec!["std::vec::Vec::push".to_string()]
    );
}

#[test]
fn recursion_unresolved_dispatch_and_missing_bodies_stay_unknown() {
    let a = Shape::new(&[])
        .calls(vec![call("w::b", Vec::new())])
        .program();
    let b = Shape::new(&[])
        .calls(vec![call("w::a", Vec::new())])
        .program();
    let owners = || vec![("w::a", a.clone()), ("w::b", b.clone())];
    assert!(unknown(compose(owners(), "w::a")).contains("Recursive"));
    assert!(unknown(compose(owners(), "w::b")).contains("Recursive"));

    let dispatch = Shape::new(&[]).calls(vec![call("?", Vec::new())]).program();
    assert!(unknown(compose(vec![("w::d", dispatch)], "w::d")).contains("Unresolved"));

    let missing = Shape::new(&[])
        .calls(vec![call("dep::f", Vec::new())])
        .program();
    assert!(unknown(compose(vec![("w::m", missing)], "w::m")).contains("No captured summary"));

    let unknown_local = Shape::new(&["n"]).scalars().scan("n").program();
    let mut unknown_local = unknown_local;
    unknown_local.nodes[2].action = Action::Unsupported { reason: "x".into() };
    let caller = Shape::new(&[])
        .calls(vec![call("w::u", Vec::new())])
        .program();
    let reason = unknown(compose(
        vec![("w::u", unknown_local), ("w::c", caller)],
        "w::c",
    ));
    assert!(reason.contains("Local bounds of w::u"), "{reason}");
}

#[test]
fn unmappable_callee_inputs_stay_unknown_but_unused_ones_do_not() {
    let local = Shape::new(&["len(v)#9"])
        .extent("len(v)#9", Root::Local(9), Vec::new())
        .scan("len(v)#9")
        .program();
    let caller = Shape::new(&[])
        .calls(vec![call("w::local", Vec::new())])
        .program();
    let reason = unknown(compose(vec![("w::local", local), ("w::c", caller)], "w::c"));
    assert!(reason.contains("callee local"), "{reason}");

    let opaque = Shape::new(&[])
        .calls(vec![call(
            "w::linear",
            vec![Argument::Scalar(Value::Unknown)],
        )])
        .program();
    let reason = unknown(compose(
        vec![("w::linear", linear()), ("w::c", opaque)],
        "w::c",
    ));
    assert!(reason.contains("no upper bound"), "{reason}");

    let constant = Shape::new(&["n"]).scalars().program();
    let caller = Shape::new(&[])
        .calls(vec![call("w::k", vec![Argument::Opaque])])
        .program();
    let result = proven(compose(vec![("w::k", constant), ("w::c", caller)], "w::c"));
    assert_eq!(result.loop_body_entries, Bound::Const(0));

    let not_shared = Shape::new(&[])
        .formals(vec![Formal {
            name: "x".into(),
            measure: Measure::Opaque,
        }])
        .calls(vec![call(
            "w::walk",
            vec![Argument::Place {
                root: Root::Formal(0),
                fields: Vec::new(),
            }],
        )])
        .program();
    let walk = Shape::new(&["len(xs)#2"])
        .extent("len(xs)#2", Root::Formal(0), Vec::new())
        .scan("len(xs)#2")
        .program();
    let reason = unknown(compose(
        vec![("w::walk", walk), ("w::c", not_shared)],
        "w::c",
    ));
    assert!(reason.contains("not a shared reference"), "{reason}");
}

#[test]
fn composed_expressions_respect_the_symbolic_budget() {
    let caller = Shape::new(&["m"])
        .scalars()
        .scan("m")
        .calls(
            (0..8)
                .map(|_| {
                    call(
                        "w::linear",
                        vec![Argument::Scalar(Value::Input("m".into()))],
                    )
                })
                .collect(),
        )
        .program();
    let limits = Limits {
        expression_nodes: 12,
        ..Limits::default()
    };
    let composed = compose_with(
        vec![("w::linear", linear()), ("w::caller", caller)],
        "w::caller",
        limits,
    );
    assert!(
        matches!(composed, Composed::BudgetExceeded { .. }),
        "{composed:?}"
    );
}

#[test]
fn assume_mode_charges_unsummarizable_callees_and_qualifies_unmapped_sizes() {
    let assume = Limits {
        opaque_loops: true,
        ..Limits::default()
    };
    let a = Shape::new(&[])
        .calls(vec![call("w::b", Vec::new())])
        .program();
    let b = Shape::new(&[])
        .calls(vec![call("w::a", Vec::new())])
        .program();
    let result = proven(compose_with(vec![("w::a", a), ("w::b", b)], "w::a", assume));
    assert!(render_inputs(&result.loop_body_entries).contains("?call(w::a)"));

    let caller = Shape::new(&["m"])
        .scalars()
        .scan("m")
        .calls(vec![
            call("?", Vec::new()),
            call("w::linear", vec![Argument::Scalar(Value::Unknown)]),
        ])
        .program();
    let result = proven(compose_with(
        vec![("w::linear", linear()), ("w::caller", caller)],
        "w::caller",
        assume,
    ));
    let inputs = render_inputs(&result.loop_body_entries);
    assert!(inputs.contains("?call(?)"), "{inputs}");
    assert!(inputs.contains("w::linear::n"), "{inputs}");
}

fn render_inputs(bound: &Bound) -> String {
    match bound {
        Bound::Input(name) => name.clone(),
        Bound::Const(_) => String::new(),
        Bound::Add(items) | Bound::Mul(items) | Bound::Max(items) => items
            .iter()
            .map(render_inputs)
            .collect::<Vec<_>>()
            .join(" "),
    }
}
