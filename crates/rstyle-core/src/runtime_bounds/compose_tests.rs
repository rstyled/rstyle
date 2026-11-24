use super::compose::{Analyzed, Composed, Composer};
use super::*;
use crate::wording::Wording;
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
        operation: None,
    }
}

/// A standard-library call keyed by its language-neutral operation.
fn library(callee: &str, operation: Option<&str>, arguments: Vec<Argument>) -> Action {
    Action::Call {
        callee: callee.into(),
        target: Callee::Library(callee.into()),
        arguments,
        may_modify: false,
        operation: operation.map(Into::into),
    }
}

fn shared(name: &str) -> Formal {
    Formal {
        name: name.into(),
        measure: Measure::Shared,
    }
}

fn scalar(name: &str) -> Formal {
    Formal {
        name: name.into(),
        measure: Measure::Scalar { input: name.into() },
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
    signed: bool,
}

impl Shape {
    fn new(inputs: &[&'static str]) -> Self {
        Self {
            inputs: inputs.to_vec(),
            formals: Vec::new(),
            extents: Vec::new(),
            limit: None,
            calls: Vec::new(),
            signed: false,
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

    /// Every input and the counter are signed.
    fn signed(mut self) -> Self {
        self.signed = true;
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
            signed: self.signed,
        });
        let counter = Variable {
            name: "i".into(),
            initial: Value::Unknown,
            bits: 64,
            signed: self.signed,
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
    composer.composed(root, Wording::of("rust"))
}

fn proven(composed: Composed) -> compose::Composition {
    match composed {
        Composed::Proven(composition) => composition,
        other => panic!("{other:?}"),
    }
}

fn partial(composed: Composed) -> compose::Composition {
    match composed {
        Composed::Partial(composition) => composition,
        other => panic!("{other:?}"),
    }
}

/// The single opaque factor of `kind`, checked against the bounds it occurs in.
/// An opaque callee also has an `Invocations` factor at the same site.
fn opaque(composition: &compose::Composition, kind: OpaqueKind) -> (String, String) {
    let found: Vec<_> = composition
        .opaque_factors
        .iter()
        .filter(|factor| factor.kind == kind)
        .collect();
    let [factor] = found.as_slice() else {
        panic!("{composition:?}");
    };
    assert!(!composition.loop_body_entries.is_complete());
    if kind == OpaqueKind::Call {
        assert!(!composition.workspace_invocations.is_complete());
        assert!(
            composition.opaque_factors.iter().any(|other| {
                other.kind == OpaqueKind::Invocations && other.site == factor.site
            })
        );
    }
    (factor.site.clone(), factor.reason.clone())
}

fn eval(bound: &Bound, values: &[(&str, u64)]) -> u64 {
    let lookup = |name: &str| values.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
    bound
        .evaluate(&lookup)
        .unwrap_or_else(|| panic!("{bound} has an unbound input or opaque factor"))
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
fn recursion_unresolved_dispatch_and_missing_bodies_are_opaque_calls() {
    let a = Shape::new(&[])
        .calls(vec![call("w::b", Vec::new())])
        .program();
    let b = Shape::new(&[])
        .calls(vec![call("w::a", Vec::new())])
        .program();
    let owners = || vec![("w::a", a.clone()), ("w::b", b.clone())];
    let cycle = partial(compose(owners(), "w::a"));
    let (site, reason) = opaque(&cycle, OpaqueKind::Call);
    assert_eq!(site, "w::a");
    // Only summarized callees are resolved; the recursive one is charged opaquely.
    assert_eq!(cycle.resolved_callees, vec!["w::b".to_string()]);
    assert!(reason.contains("Recursive"), "{reason}");
    let (_, reason) = opaque(&partial(compose(owners(), "w::b")), OpaqueKind::Call);
    assert!(reason.contains("Recursive"), "{reason}");

    let dispatch = Shape::new(&[]).calls(vec![call("?", Vec::new())]).program();
    let dispatched = partial(compose(vec![("w::d", dispatch)], "w::d"));
    let (_, reason) = opaque(&dispatched, OpaqueKind::Call);
    assert!(reason.contains("Unresolved"), "{reason}");

    let missing = Shape::new(&[])
        .calls(vec![call("dep::f", Vec::new())])
        .program();
    let (_, reason) = opaque(
        &partial(compose(vec![("w::m", missing)], "w::m")),
        OpaqueKind::Call,
    );
    assert!(reason.contains("No captured summary"), "{reason}");
}

/// An owner whose only work is calling `target`.
fn calling(target: &str) -> Program {
    Shape::new(&[])
        .calls(vec![call(target, Vec::new())])
        .program()
}

/// What each owner of `order` composes to when one composer composes them in that order.
fn composed_in_turn(
    owners: &[(String, Program)],
    order: &[&str],
    limits: Limits,
) -> BTreeMap<String, Composed> {
    let owners: BTreeMap<&str, &Program> = owners
        .iter()
        .map(|(id, program)| (id.as_str(), program))
        .collect();
    let mut composer = Composer::new(
        |id: &str| {
            owners.get(id).map(|program| Analyzed {
                outcome: analyze(program),
                program: (*program).clone(),
            })
        },
        limits,
    );
    order
        .iter()
        .map(|id| (id.to_string(), composer.composed(id, Wording::of("rust"))))
        .collect()
}

/// `w::0` calls `w::1` and so on to `w::{last}`, which calls `w::0` when `ring`.
fn chain(last: usize, ring: bool) -> Vec<(String, Program)> {
    (0..=last)
        .map(|at| {
            let program = match (at == last, ring) {
                (false, _) => calling(&format!("w::{}", at + 1)),
                (true, true) => calling("w::0"),
                (true, false) => Shape::new(&[]).program(),
            };
            (format!("w::{at}"), program)
        })
        .collect()
}

#[test]
fn an_owner_composes_alike_whichever_owner_was_composed_first() {
    // `w::0`, `w::1` and `w::2` call each other in a ring, and `w::3` calls into it.
    let mut owners = chain(2, true);
    owners.push(("w::3".into(), calling("w::0")));
    let forward = composed_in_turn(
        &owners,
        &["w::0", "w::1", "w::2", "w::3"],
        Limits::default(),
    );
    // Other orders enter the ring at `w::2` and at `w::1`; `w::3` enters it at `w::0`.
    for order in [
        ["w::2", "w::1", "w::0", "w::3"],
        ["w::3", "w::1", "w::2", "w::0"],
    ] {
        assert_eq!(
            composed_in_turn(&owners, &order, Limits::default()),
            forward
        );
    }

    // Each owner of the ring walks round it, to the call back into itself.
    for (id, others) in [
        ("w::0", ["w::1", "w::2"]),
        ("w::1", ["w::0", "w::2"]),
        ("w::2", ["w::0", "w::1"]),
    ] {
        let composition = partial(forward[id].clone());
        assert_eq!(composition.resolved_callees, others);
        let (site, reason) = opaque(&composition, OpaqueKind::Call);
        assert_eq!(site, id);
        assert!(reason.contains("Recursive call cycle"), "{reason}");
    }
    let into = partial(forward["w::3"].clone());
    assert_eq!(into.resolved_callees, ["w::0", "w::1", "w::2"]);
}

#[test]
fn each_owner_has_a_budget_of_its_own_spent_walking_round_its_cycles() {
    // 16 owners walked through in all.
    let limits = Limits {
        nodes: 1,
        ..Limits::default()
    };
    // A chain has no cycle to walk round: however long, every owner of it composes in full.
    let line = chain(39, false);
    let ids: Vec<String> = (0..40).map(|at| format!("w::{at}")).collect();
    let order: Vec<&str> = ids.iter().map(String::as_str).collect();
    let composed = composed_in_turn(&line, &order, limits);
    let first = proven(composed["w::0"].clone());
    assert_eq!(first.resolved_callees.len(), 39);
    assert!(
        composed
            .values()
            .all(|it| matches!(it, Composed::Proven(_)))
    );

    // Round a ring longer than the budget, each owner's walk is cut where its budget runs out,
    // whichever owner was composed first.
    let ring = chain(19, true);
    let reversed: Vec<&str> = order[..20].iter().rev().copied().collect();
    let forward = composed_in_turn(&ring, &order[..20], limits);
    assert_eq!(forward, composed_in_turn(&ring, &reversed, limits));
    let cut = partial(forward["w::0"].clone());
    assert_eq!(cut.resolved_callees.len(), 15);
    let (site, reason) = opaque(&cut, OpaqueKind::Call);
    assert_eq!(site, "w::16");
    assert_eq!(reason, "Composition owner budget exceeded");
}

#[test]
fn unknown_callees_multiply_their_opaque_work_by_the_call_count() {
    let mut unknown_local = Shape::new(&["n"]).scalars().scan("n").program();
    unknown_local.nodes[2].action = Action::Unsupported { reason: "x".into() };
    let caller = Shape::new(&["m"])
        .scalars()
        .scan("m")
        .calls(vec![call("w::u", Vec::new())])
        .program();
    let owners = vec![("w::u", unknown_local.clone()), ("w::c", caller)];
    let result = partial(compose(owners, "w::c"));
    let (_, reason) = opaque(&result, OpaqueKind::Call);
    assert!(reason.contains("Local bounds of w::u"), "{reason}");
    // The proven caller loop still multiplies the opaque callee: m + m * ?call(w::u).
    assert_eq!(
        result.loop_body_entries,
        Bound::Add(vec![
            Bound::Input("m".into()),
            Bound::Mul(vec![
                Bound::Input("m".into()),
                Bound::opaque(OpaqueKind::Call, "w::u")
            ]),
        ])
    );

    // An owner whose own local bounds are unknown has no composed bound.
    assert!(matches!(
        compose(vec![("w::u", unknown_local)], "w::u"),
        Composed::Unknown { .. }
    ));
}

#[test]
fn unmappable_callee_inputs_are_opaque_but_unused_ones_are_not() {
    let local = Shape::new(&["len(v)#9"])
        .extent("len(v)#9", Root::Local(9), Vec::new())
        .scan("len(v)#9")
        .program();
    let caller = Shape::new(&[])
        .calls(vec![call("w::local", Vec::new())])
        .program();
    let result = partial(compose(vec![("w::local", local), ("w::c", caller)], "w::c"));
    let (site, reason) = opaque(&result, OpaqueKind::Input);
    assert_eq!(site, "w::local::len(v)#9");
    assert!(reason.contains("callee local"), "{reason}");

    let unbounded = Shape::new(&[])
        .calls(vec![call(
            "w::linear",
            vec![Argument::Scalar(Value::Unknown)],
        )])
        .program();
    let owners = vec![("w::linear", linear()), ("w::c", unbounded)];
    let result = partial(compose(owners, "w::c"));
    let (site, reason) = opaque(&result, OpaqueKind::Input);
    assert_eq!(site, "w::linear::n");
    assert!(reason.contains("no upper bound"), "{reason}");

    let constant = Shape::new(&["n"]).scalars().program();
    let caller = Shape::new(&[])
        .calls(vec![call("w::k", vec![Argument::Opaque])])
        .program();
    let result = proven(compose(vec![("w::k", constant), ("w::c", caller)], "w::c"));
    assert_eq!(result.loop_body_entries, Bound::Const(0));
}

#[test]
fn lengths_under_unshared_formals_are_opaque() {
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
    let result = partial(compose(
        vec![("w::walk", walk), ("w::c", not_shared)],
        "w::c",
    ));
    let (_, reason) = opaque(&result, OpaqueKind::Input);
    assert!(reason.contains("not a shared reference"), "{reason}");
}

/// A caller scanning `m` that calls `w::linear` once per item with each of `arguments`.
fn calling_linear_with(inputs: &[&'static str], arguments: &[&str]) -> Program {
    let calls = arguments.iter().map(|argument| {
        let argument = Argument::Scalar(Value::Input((*argument).into()));
        call("w::linear", vec![argument])
    });
    Shape::new(inputs)
        .scalars()
        .scan("m")
        .calls(calls.collect())
        .program()
}

#[test]
fn composed_expressions_respect_the_symbolic_budget() {
    // Each call passes another input, so no two calls' terms are alike.
    let inputs = ["m", "a", "b", "c", "d", "e", "f", "g", "h"];
    let caller = calling_linear_with(&inputs, &inputs[1..]);
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
fn calls_alike_compose_to_one_term_scaled_by_how_many_there_are() {
    let caller = calling_linear_with(&["m"], &["m"; 8]);
    let limits = Limits {
        expression_nodes: 12,
        ..Limits::default()
    };
    let composed = proven(compose_with(
        vec![("w::linear", linear()), ("w::caller", caller)],
        "w::caller",
        limits,
    ));
    assert_eq!(composed.loop_body_entries.to_string(), "m + 8 * m * m");
    assert_eq!(composed.workspace_invocations.to_string(), "8 * m");
    assert_eq!(eval(&composed.loop_body_entries, &[("m", 3)]), 3 + 8 * 9);
}

/// `fn lookup(n: usize, map: &BTreeMap<..>) { for _ in 0..n { map.get(..); } }`
fn lookups() -> Program {
    lookups_and(Vec::new())
}

fn lookups_and(mut calls: Vec<Action>) -> Program {
    let get = library(
        "std::collections::BTreeMap::get",
        Some("rust:BTreeMap.get"),
        vec![
            Argument::Place {
                root: Root::Formal(1),
                fields: Vec::new(),
            },
            Argument::Opaque,
        ],
    );
    Shape::new(&["n"])
        .formals(vec![scalar("n"), shared("map")])
        .scan("n")
        .calls({
            calls.insert(0, get);
            calls
        })
        .program()
}

#[test]
fn contracted_lookups_compose_logarithmic_library_steps() {
    let own = proven(compose(vec![("w::lookup", lookups())], "w::lookup"));
    assert_eq!(own.library_steps.to_string(), "n * log(len(map))");
    assert!(own.library_factors.is_empty());
    assert_eq!(
        own.excluded_library_callees,
        ["std::collections::BTreeMap::get"]
    );

    // `fn caller(ctx: &Ctx, k: usize) { lookup(k, &ctx.index) }`
    let caller = Shape::new(&["k"])
        .formals(vec![shared("ctx"), scalar("k")])
        .calls(vec![call(
            "w::lookup",
            vec![
                Argument::Scalar(Value::Input("k".into())),
                Argument::Place {
                    root: Root::Formal(0),
                    fields: vec![field(0, "index")],
                },
            ],
        )])
        .program();
    let composed = proven(compose(
        vec![("w::lookup", lookups()), ("w::caller", caller)],
        "w::caller",
    ));
    assert_eq!(
        composed.library_steps.to_string(),
        "k * log(len(ctx.index))"
    );
    let steps = eval(
        &composed.library_steps,
        &[("k", 10), ("len(ctx.index)", 1000)],
    );
    assert_eq!(steps, 100);
    assert_eq!(composed.library_steps.order().logarithmic, 1);
}

#[test]
fn library_steps_without_contracts_or_sizes_stay_opaque_without_changing_the_verdict() {
    let local = Argument::Place {
        root: Root::Local(7),
        fields: Vec::new(),
    };
    let owner = Shape::new(&[])
        .calls(vec![
            library("std::vec::Vec::push", Some("rust:Vec.push"), Vec::new()),
            library("std::mem::swap", None, Vec::new()),
            library(
                "std::collections::BTreeSet::contains",
                Some("rust:BTreeSet.contains"),
                vec![local],
            ),
            library("std::vec::Vec::len", Some("rust:Vec.len"), Vec::new()),
            call("ctor", Vec::new()),
        ])
        .program();
    let composition = proven(compose(vec![("w::f", owner)], "w::f"));
    assert!(composition.opaque_factors.is_empty());
    assert_eq!(
        composition.library_steps.to_string(),
        "?library(std::vec::Vec::push) + ?library(std::mem::swap) + \
         log(?input(std::collections::BTreeSet::contains::size(0))) + 1"
    );
    let reasons: Vec<_> = composition
        .library_factors
        .iter()
        .map(|factor| (factor.kind, factor.reason.as_str()))
        .collect();
    assert_eq!(
        reasons,
        [
            (
                OpaqueKind::Input,
                "Callee input size(0) at call to std::collections::BTreeSet::contains is unmapped: caller place is not a stable extent"
            ),
            (OpaqueKind::Library, "Library call has no operation key"),
            (OpaqueKind::Library, "No library contract for rust:Vec.push"),
        ]
    );
}

#[test]
fn unsummarized_callees_charge_opaque_library_steps() {
    let caller = Shape::new(&["n"])
        .scan("n")
        .calls(vec![call("?", Vec::new()), call("w::lookup", Vec::new())])
        .program();
    let composition = partial(compose(
        vec![("w::caller", caller), ("w::lookup", lookups())],
        "w::caller",
    ));
    let sites: Vec<_> = composition
        .library_factors
        .iter()
        .map(|factor| (factor.kind, factor.site.as_str()))
        .collect();
    // The lookup's scalar size is unmapped, so its count is opaque; its map size too.
    assert!(sites.contains(&(OpaqueKind::Library, "?")), "{sites:?}");
    assert!(
        sites.contains(&(OpaqueKind::Input, "w::lookup::len(map)")),
        "{sites:?}"
    );
    assert!(!composition.library_steps.is_complete());
}

#[test]
fn injected_contracts_replace_the_embedded_table() {
    let contracts = compose::Contracts::parse_all(&[r#"
        schema = "rstyle.library-contracts/1"
        namespace = "rust"
        [[contract]]
        basis = "test"
        steps = { Mul = [{ Size = 0 }, { Size = 0 }] }
        operations = ["BTreeMap.get"]
        [[contract]]
        basis = "scalar sizes are their values"
        steps = { Size = 1 }
        operations = ["Vec.reserve"]
    "#])
    .unwrap();
    // `out.reserve(n)` per lookup too: a scalar argument's size is its bound.
    let program = lookups_and(vec![library(
        "std::vec::Vec::reserve",
        Some("rust:Vec.reserve"),
        vec![Argument::Opaque, Argument::Scalar(Value::Input("n".into()))],
    )]);
    let mut composer = Composer::with_contracts(
        |_: &str| {
            Some(Analyzed {
                outcome: analyze(&program),
                program: program.clone(),
            })
        },
        Limits::default(),
        contracts,
    );
    let composition = proven(composer.composed("w::lookup", Wording::of("rust")));
    assert_eq!(
        composition.library_steps.to_string(),
        "n * len(map) * len(map) + n * n"
    );
}

fn signed_callee() -> Program {
    Shape::new(&["k"]).scalars().signed().scan("k").program()
}

fn signed_caller(argument: Value) -> Program {
    Shape::new(&["m"])
        .scalars()
        .signed()
        .calls(vec![call("w::f", vec![Argument::Scalar(argument)])])
        .program()
}

#[test]
fn a_signed_callee_stays_floored_at_its_callers_argument() {
    let composed = |caller| {
        let owners = vec![("w::f", signed_callee()), ("w::c", caller)];
        compose(owners, "w::c")
    };
    let input = proven(composed(signed_caller(Value::Input("m".into()))));
    assert_eq!(input.loop_body_entries.to_string(), "max(m, 0)");

    let constant = proven(composed(signed_caller(Value::Const(7))));
    assert_eq!(constant.loop_body_entries, Bound::Const(7));

    // A negative argument lowers to an unknown value, which is opaque and not `k`.
    let negative = partial(composed(signed_caller(Value::Unknown)));
    assert_eq!(opaque(&negative, OpaqueKind::Input).0, "w::f::k");
}

#[test]
fn a_signed_counter_argument_is_floored_at_the_callers_limit() {
    let caller = Shape::new(&["m"]).scalars().signed().scan("m");
    let counter = Value::Variable(caller.counter());
    let caller = caller
        .calls(vec![call("w::f", vec![Argument::Scalar(counter)])])
        .program();
    let result = proven(compose(
        vec![("w::f", signed_callee()), ("w::c", caller)],
        "w::c",
    ));
    assert_eq!(
        result.loop_body_entries.to_string(),
        "max(m, 0) + max(m, 0) * max(m, 0)"
    );
}
