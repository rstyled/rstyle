use super::*;

fn node(action: Action, flow: Flow) -> Node {
    Node {
        action,
        flow,
        source: None,
    }
}

fn guard(variable: u32, limit: u32, body: u32, exit: u32) -> Node {
    node(
        Action::Nop,
        Flow::Branch {
            condition: Condition::Less {
                variable,
                limit: Value::Variable(limit),
            },
            then_node: body,
            else_node: exit,
        },
    )
}

fn assign(variable: u32, next: u32) -> Node {
    node(
        Action::Assign {
            variable,
            value: Value::Const(0),
        },
        Flow::Goto(next),
    )
}

fn opaque_call(callee: impl Into<String>, may_modify: bool) -> Action {
    Action::Call {
        callee: callee.into(),
        target: Callee::Unresolved,
        arguments: Vec::new(),
        may_modify,
        operation: None,
    }
}

fn increment(variable: u32, next: u32) -> Node {
    node(Action::Increment { variable }, Flow::Goto(next))
}

fn scan() -> Program {
    Program {
        inputs: vec!["n".into(), "m".into()],
        variables: vec![
            Variable {
                name: "n".into(),
                initial: Value::Input("n".into()),
                bits: 32,
                signed: false,
            },
            Variable {
                name: "m".into(),
                initial: Value::Input("m".into()),
                bits: 32,
                signed: false,
            },
            Variable {
                name: "i".into(),
                initial: Value::Unknown,
                bits: 32,
                signed: false,
            },
            Variable {
                name: "j".into(),
                initial: Value::Unknown,
                bits: 32,
                signed: false,
            },
        ],
        nodes: vec![
            assign(2, 1),
            guard(2, 0, 2, 4),
            node(opaque_call("visit", false), Flow::Goto(3)),
            increment(2, 1),
            node(Action::Nop, Flow::Return),
        ],
        entry: 0,
        loop_headers: vec![1],
        arithmetic: Arithmetic::Checked,
        formals: Vec::new(),
        extents: Vec::new(),
    }
}

fn nested(reset: bool) -> Program {
    let mut program = scan();
    program.nodes = vec![
        assign(2, 1),
        assign(3, 2),
        guard(2, 0, 3, 8),
        if reset {
            assign(3, 4)
        } else {
            node(Action::Nop, Flow::Goto(4))
        },
        guard(3, 1, 5, 7),
        increment(3, 4),
        node(Action::Nop, Flow::Return),
        increment(2, 2),
        node(Action::Nop, Flow::Return),
    ];
    program.loop_headers = vec![2, 4];
    program
}

fn proven(program: &Program) -> Assessment {
    let Outcome::Proven(result) = analyze(program) else {
        panic!("{:?}", analyze(program));
    };
    assert!(result.whole_runtime.starts_with("unknown:"));
    result
}

fn bound_value(bound: &Bound, n: u64, m: u64) -> u64 {
    // Every input other than `n` is `m`, as in the traced programs.
    let value = |name: &str| Some(if name == "n" { n } else { m });
    bound
        .evaluate(&value)
        .unwrap_or_else(|| panic!("opaque factor in {bound}"))
}

fn trace(program: &Program, n: u64, m: u64) -> u64 {
    let mut values = [n, m, 0, 0];
    let mut position = program.entry;
    let mut entries = 0;
    for _ in 0..1000 {
        let current = &program.nodes[position as usize];
        match current.action {
            Action::Assign {
                variable,
                value: Value::Const(value),
            } => values[variable as usize] = value,
            Action::Increment { variable } => values[variable as usize] += 1,
            _ => {}
        }
        position = match &current.flow {
            Flow::Goto(next) => *next,
            Flow::Return => return entries,
            Flow::Branch {
                condition:
                    Condition::Less {
                        variable,
                        limit: Value::Variable(limit),
                    },
                then_node,
                else_node,
            } => {
                if values[*variable as usize] < values[*limit as usize] {
                    entries += u64::from(program.loop_headers.contains(&position));
                    *then_node
                } else {
                    *else_node
                }
            }
            _ => panic!("unsupported oracle input"),
        };
    }
    panic!("trace budget exceeded")
}

#[test]
fn scalar_scan_counts_entries_and_calls_without_claiming_runtime() {
    let result = proven(&scan());
    assert_eq!(result.loop_body_entries, Bound::Input("n".into()));
    assert_eq!(result.calls[0].count, Bound::Input("n".into()));
}

#[test]
fn persistent_cursor_and_per_outer_reset_have_different_global_counts() {
    let persistent = proven(&nested(false));
    let reset = proven(&nested(true));
    assert_eq!(
        persistent.loop_body_entries,
        Bound::Add(vec![Bound::Input("n".into()), Bound::Input("m".into())])
    );
    assert_eq!(
        reset.loops[1].body_entries,
        Bound::Mul(vec![Bound::Input("n".into()), Bound::Input("m".into())])
    );
    for program in [nested(false), nested(true), scan()] {
        let bound = proven(&program).loop_body_entries;
        for n in 0..6 {
            for m in 0..6 {
                assert!(trace(&program, n, m) <= bound_value(&bound, n, m));
            }
        }
    }
}

#[test]
fn continue_before_increment_and_changed_limits_are_unknown() {
    let mut skipped = scan();
    skipped.nodes[2].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 1,
        else_node: 3,
    };
    assert!(unproven(&skipped));
    let mut limit = scan();
    limit.nodes[2].action = Action::Assign {
        variable: 0,
        value: Value::Const(0),
    };
    assert!(unproven(&limit));
}

#[test]
fn unsupported_predicates_aliasing_and_missing_widths_do_not_certify() {
    let mut predicate = scan();
    if let Flow::Branch { condition, .. } = &mut predicate.nodes[1].flow {
        *condition = Condition::Unknown;
    }
    let mut alias = scan();
    alias.nodes[2].action = opaque_call("unknown", true);
    let mut width = scan();
    width.variables[2].bits = 8;
    for program in [predicate, alias, width] {
        assert!(unproven(&program));
    }
}

#[test]
fn budgets_never_publish_partial_bounds() {
    let limits = Limits {
        nodes: 2,
        ..Limits::default()
    };
    assert!(matches!(
        analyze_with_limits(&scan(), limits),
        Outcome::BudgetExceeded { .. }
    ));
    let limits = Limits {
        iterations: 0,
        ..Limits::default()
    };
    assert!(matches!(
        analyze_with_limits(&scan(), limits),
        Outcome::BudgetExceeded { .. }
    ));
}

#[test]
fn wrapping_increments_are_certified_only_under_current_state_guard() {
    let mut program = scan();
    program.arithmetic = Arithmetic::Wrapping;
    program.variables[0].initial = Value::Const(u32::MAX as u64);
    assert_eq!(
        proven(&program).loop_body_entries,
        Bound::Const(u32::MAX as u64)
    );
    program.nodes[2].action = Action::Increment { variable: 2 };
    assert!(unproven(&program));
}

#[test]
fn sequential_independent_scans_add_their_counts() {
    let mut program = scan();
    program.nodes[4] = assign(3, 5);
    program.nodes.extend([
        guard(3, 1, 6, 8),
        node(Action::Nop, Flow::Goto(7)),
        increment(3, 5),
        node(Action::Nop, Flow::Return),
    ]);
    program.loop_headers.push(5);
    let bound = proven(&program).loop_body_entries;
    assert_eq!(
        bound,
        Bound::Add(vec![Bound::Input("n".into()), Bound::Input("m".into())])
    );
    for n in 0u64..5 {
        for m in 0..5 {
            assert_eq!(trace(&program, n, m), bound_value(&bound, n, m));
        }
    }
}

#[test]
fn continue_after_increment_and_simple_early_exit_preserve_upper_bound() {
    let mut program = scan();
    program.nodes[3].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 1,
        else_node: 4,
    };
    assert_eq!(proven(&program).loop_body_entries, Bound::Input("n".into()));
    let mut early = scan();
    early.nodes[2].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 3,
        else_node: 4,
    };
    assert_eq!(proven(&early).loop_body_entries, Bound::Input("n".into()));
}

#[test]
fn persistent_cursor_with_unproductive_early_exits_is_not_amortized() {
    let mut program = nested(false);
    program.nodes[5] = node(
        Action::Nop,
        Flow::Branch {
            condition: Condition::Unknown,
            then_node: 6,
            else_node: 7,
        },
    );
    program.nodes[6] = increment(3, 4);
    assert!(unproven(&program));
}

#[test]
fn repeated_outer_increment_inside_inner_cycle_is_not_a_guarded_scan() {
    let mut program = nested(false);
    program.nodes[5] = increment(2, 9);
    program.nodes[7] = node(Action::Nop, Flow::Goto(2));
    program.nodes.push(increment(3, 4));
    assert!(unproven(&program));
}

#[test]
fn saved_limit_never_reads_a_later_assignment_as_its_entry_snapshot() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "saved".into(),
        initial: Value::Unknown,
        bits: 32,
        signed: false,
    });
    program.nodes = vec![
        assign(2, 1),
        node(
            Action::Assign {
                variable: 4,
                value: Value::Variable(0),
            },
            Flow::Goto(2),
        ),
        node(
            Action::Assign {
                variable: 0,
                value: Value::Const(0),
            },
            Flow::Goto(3),
        ),
        guard(2, 4, 4, 6),
        node(Action::Nop, Flow::Goto(5)),
        increment(2, 3),
        node(Action::Nop, Flow::Return),
    ];
    program.loop_headers = vec![3];
    assert!(unproven(&program));
}

#[test]
fn nonzero_unsigned_range_start_retains_conservative_end_bound() {
    let mut program = scan();
    program.nodes[0].action = Action::Assign {
        variable: 2,
        value: Value::Variable(1),
    };
    assert_eq!(proven(&program).loop_body_entries, Bound::Input("n".into()));
    for n in 0u64..5 {
        for m in 0..5 {
            assert!(n.saturating_sub(m) <= bound_value(&proven(&program).loop_body_entries, n, m));
        }
    }
}

#[test]
fn captures_with_no_backedge_do_not_invent_zero_entry_counts() {
    let mut program = scan();
    program.nodes[2].flow = Flow::Return;
    // The body is entered once before returning; a missing cycle is not zero entries.
    assert_eq!(proven(&program).loop_body_entries, Bound::Const(1));

    let mut nested = nested(true);
    nested.nodes[5].flow = Flow::Goto(7);
    let result = proven(&nested);
    assert_eq!(result.loops[1].body_entries, Bound::Input("n".into()));
}

#[test]
fn initializer_assignments_can_supply_an_immutable_limit() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "length".into(),
        initial: Value::Unknown,
        bits: 32,
        signed: false,
    });
    program.nodes.push(node(
        Action::Assign {
            variable: 4,
            value: Value::Variable(0),
        },
        Flow::Goto(0),
    ));
    program.entry = 5;
    program.nodes[1] = guard(2, 4, 2, 4);
    assert_eq!(proven(&program).loop_body_entries, Bound::Input("n".into()));
}

#[test]
fn saved_end_after_completed_counter_loop_is_not_its_zero_initializer() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "end".into(),
        initial: Value::Unknown,
        bits: 32,
        signed: false,
    });
    program.nodes[4] = node(
        Action::Assign {
            variable: 4,
            value: Value::Variable(2),
        },
        Flow::Goto(5),
    );
    program.nodes.extend([
        assign(3, 6),
        guard(3, 4, 7, 8),
        increment(3, 6),
        node(Action::Nop, Flow::Return),
    ]);
    program.loop_headers.push(6);
    assert!(unproven(&program));
}

#[test]
fn unrelated_accumulator_increments_do_not_change_private_scan_bounds() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "total".into(),
        initial: Value::Const(0),
        bits: 8,
        signed: false,
    });
    program.nodes[2] = increment(4, 3);
    assert_eq!(proven(&program).loop_body_entries, Bound::Input("n".into()));
    program.arithmetic = Arithmetic::Wrapping;
    assert_eq!(proven(&program).loop_body_entries, Bound::Input("n".into()));
}

// Lowered shape of `outer.map(|_| inner.filter(p).map(score).sum())`: the inlined inner
// scan resets per outer item, the filter skips to the inner increment, and a search
// consumer may stop the whole chain from inside the inner scan.
fn callback_scan(early_exit: bool) -> Program {
    let mut program = scan();
    let call = |callee: &str, next| node(opaque_call(callee, false), Flow::Goto(next));
    program.nodes = vec![
        assign(2, 1),
        guard(2, 0, 2, 9),
        assign(3, 3),
        guard(3, 1, 4, 7),
        node(
            Action::Nop,
            Flow::Branch {
                condition: Condition::Unknown,
                then_node: 5,
                else_node: if early_exit { 9 } else { 6 },
            },
        ),
        call("score", 6),
        increment(3, 3),
        call("key", 8),
        increment(2, 1),
        node(Action::Nop, Flow::Return),
    ];
    program.loop_headers = vec![1, 3];
    program
}

fn call_count(result: &Assessment, callee: &str) -> Bound {
    result
        .calls
        .iter()
        .find(|call| call.callee == callee)
        .unwrap_or_else(|| panic!("missing {callee}: {result:?}"))
        .count
        .clone()
}

#[test]
fn inlined_callback_scans_multiply_independent_cardinalities() {
    let result = proven(&callback_scan(false));
    let product = Bound::Mul(vec![Bound::Input("n".into()), Bound::Input("m".into())]);
    assert_eq!(call_count(&result, "score"), product);
    assert_eq!(call_count(&result, "key"), Bound::Input("n".into()));
    assert_eq!(
        result.loop_body_entries,
        Bound::Add(vec![Bound::Input("n".into()), product])
    );
}

#[test]
fn short_circuit_from_a_nested_callback_scan_keeps_the_reset_bound() {
    let result = proven(&callback_scan(true));
    assert_eq!(
        call_count(&result, "score"),
        Bound::Mul(vec![Bound::Input("n".into()), Bound::Input("m".into())])
    );
}

#[test]
fn callbacks_that_write_a_scan_limit_or_skip_the_reset_stay_unknown() {
    let mut writes_limit = callback_scan(false);
    writes_limit.nodes[5].action = Action::Assign {
        variable: 1,
        value: Value::Unknown,
    };
    assert!(unproven(&writes_limit));

    let mut persistent = callback_scan(true);
    persistent.nodes[2].action = Action::Nop;
    persistent.nodes[0] = assign(2, 10);
    persistent.nodes.push(assign(3, 1));
    assert!(unproven(&persistent));

    let mut modifying = callback_scan(false);
    modifying.nodes[5].action = opaque_call("score", true);
    assert!(unproven(&modifying));
}

// Not certified: either unknown, or partial with an unproven loop made opaque.
fn unproven(program: &Program) -> bool {
    match analyze(program) {
        Outcome::Proven(_) => false,
        Outcome::Partial(result) => result.loops.iter().any(|item| item.opaque.is_some()),
        Outcome::Unknown { .. } | Outcome::BudgetExceeded { .. } => true,
    }
}

fn partial(program: &Program) -> Assessment {
    match analyze(program) {
        Outcome::Partial(result) => result,
        other => panic!("{other:?}"),
    }
}

fn opaque_loop(header: usize) -> Bound {
    Bound::opaque(OpaqueKind::Loop, format!("node {header}"))
}

// `while (?) { j = 0; while (j < m) { j += 1 } i += 1 }` with an unprovable outer guard.
fn opaque_outer() -> Program {
    let mut program = nested(true);
    program.nodes[2].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 3,
        else_node: 8,
    };
    program
}

#[test]
fn unprovable_loops_are_opaque_factors_in_partial_bounds() {
    let result = partial(&opaque_outer());
    assert_eq!(result.loops[0].body_entries, opaque_loop(2));
    assert!(result.loops[0].opaque.is_some());
    // The inner scan still resets per outer entry, so it multiplies the opaque factor.
    assert_eq!(
        result.loops[1].body_entries,
        Bound::Mul(vec![opaque_loop(2), Bound::Input("m".into())])
    );
    assert_eq!(result.loops[1].opaque, None);
    assert!(!result.loop_body_entries.is_complete());
}

#[test]
fn partial_bounds_keep_proven_siblings_but_not_whole_owner_failures() {
    let mut twice = scan();
    twice.nodes[2].action = Action::Increment { variable: 2 };
    let result = partial(&twice);
    assert!(result.loops[0].opaque.is_some());
    assert_eq!(result.loops[0].body_entries, opaque_loop(1));

    assert!(matches!(analyze(&nested(false)), Outcome::Proven(_)));

    let mut unsupported = opaque_outer();
    unsupported.nodes[5].action = Action::Unsupported {
        reason: "unmodeled".into(),
    };
    assert!(matches!(analyze(&unsupported), Outcome::Unknown { .. }));
}

#[test]
fn changed_limits_stray_increments_and_nested_unprovable_loops_are_opaque() {
    let mut changed = scan();
    changed.nodes[2].action = Action::Assign {
        variable: 0,
        value: Value::Const(5),
    };
    assert!(partial(&changed).loops[0].opaque.is_some());

    // The counter is incremented again after the loop exits, outside its guard.
    let mut stray = scan();
    stray.nodes[4] = increment(2, 5);
    stray.nodes.push(node(Action::Nop, Flow::Return));
    let result = partial(&stray);
    let opaque = result.loops[0].opaque.as_ref().unwrap();
    assert!(opaque.reason.contains("no-overflow"), "{opaque:?}");

    let mut inner = nested(true);
    inner.nodes[4].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 5,
        else_node: 7,
    };
    let result = partial(&inner);
    assert_eq!(result.loops[0].body_entries, Bound::Input("n".into()));
    assert_eq!(
        result.loops[1].body_entries,
        Bound::Mul(vec![Bound::Input("n".into()), opaque_loop(4)])
    );
}

#[test]
fn declared_loops_that_never_cycle_are_ignored() {
    let mut exits = scan();
    // The body returns instead of reaching its increment, so the guard never repeats.
    exits.nodes[2].flow = Flow::Goto(4);
    let Outcome::Proven(result) = analyze(&exits) else {
        panic!("{:?}", analyze(&exits));
    };
    assert_eq!(result.loops[0].body_entries, Bound::Const(1));
    assert_eq!(result.calls[0].count, Bound::Const(1));
}

#[test]
fn a_counter_shared_between_loop_guards_still_fails_the_whole_owner() {
    let mut program = scan();
    program.nodes = vec![
        assign(2, 1),
        guard(2, 0, 2, 3),
        increment(2, 1),
        assign(2, 4),
        guard(2, 1, 5, 6),
        increment(2, 4),
        node(Action::Nop, Flow::Return),
    ];
    program.loop_headers = vec![1, 4];
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
}

// `while i < n { j = 0; b = 0; while b < n { b += 1; while j < m { [break]; j += 1 } } i += 1 }`
// The reset is per outer iteration, but the middle loop re-enters the scan.
fn reset_under_middle(early_exit: bool) -> Program {
    let mut program = scan();
    program.variables.push(Variable {
        name: "b".into(),
        initial: Value::Unknown,
        bits: 32,
        signed: false,
    });
    let skip = node(
        Action::Nop,
        Flow::Branch {
            condition: Condition::Unknown,
            then_node: 4,
            else_node: 8,
        },
    );
    program.nodes = vec![
        assign(2, 1),
        guard(2, 0, 2, 10),
        assign(3, 3),
        assign(4, 4),
        guard(4, 0, 5, 9),
        increment(4, 6),
        guard(3, 1, 7, 4),
        if early_exit {
            skip
        } else {
            node(Action::Nop, Flow::Goto(8))
        },
        increment(3, 6),
        increment(2, 1),
        node(Action::Nop, Flow::Return),
    ];
    program.loop_headers = vec![1, 4, 6];
    program
}

#[test]
fn reset_scans_reentered_by_a_middle_loop_charge_early_exits() {
    let exact = proven(&reset_under_middle(false));
    let scan = exact.loops.iter().find(|item| item.header == 6).unwrap();
    assert_eq!(
        scan.body_entries,
        Bound::Mul(vec![Bound::Input("n".into()), Bound::Input("m".into())])
    );

    // Each `break` is a body entry that leaves the counter unchanged, once per
    // middle iteration: n * n entries can exceed n * m.
    let partial = partial(&reset_under_middle(true));
    let scan = partial.loops.iter().find(|item| item.header == 6).unwrap();
    assert!(scan.opaque.is_some(), "{scan:?}");
}

#[test]
fn opaque_loops_under_a_zero_trip_parent_still_make_the_owner_partial() {
    let mut program = opaque_outer();
    program.variables[0].initial = Value::Const(0);
    program.inputs.retain(|name| name != "n");
    program.nodes[2].flow = Flow::Branch {
        condition: Condition::Less {
            variable: 2,
            limit: Value::Variable(0),
        },
        then_node: 3,
        else_node: 8,
    };
    program.nodes[4].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 5,
        else_node: 7,
    };
    let result = partial(&program);
    assert_eq!(result.loop_body_entries, Bound::Const(0));

    let analyzed = compose::Analyzed {
        outcome: analyze(&program),
        program,
    };
    let mut composer = compose::Composer::new(
        |_: &str| {
            Some(compose::Analyzed {
                program: analyzed.program.clone(),
                outcome: analyzed.outcome.clone(),
            })
        },
        Limits::default(),
    );
    let compose::Composed::Partial(composed) =
        composer.composed("w::zero", crate::wording::Wording::Rust)
    else {
        panic!("a zero bound must not hide the opaque loop");
    };
    assert_eq!(composed.opaque_factors[0].kind, OpaqueKind::Loop);
}

/// `i = start; while i < n { i += 1 }` over signed 64-bit `n` and `i`.
fn signed_scan(start: Value) -> Program {
    Program {
        inputs: vec!["n".into()],
        variables: vec![
            signed("n", Value::Input("n".into())),
            signed("i", Value::Unknown),
        ],
        nodes: vec![
            node(
                Action::Assign {
                    variable: 1,
                    value: start,
                },
                Flow::Goto(1),
            ),
            guard(1, 0, 2, 3),
            increment(1, 1),
            node(Action::Nop, Flow::Return),
        ],
        entry: 0,
        loop_headers: vec![1],
        arithmetic: Arithmetic::Checked,
        formals: Vec::new(),
        extents: Vec::new(),
    }
}

fn signed(name: &str, initial: Value) -> Variable {
    Variable {
        name: name.into(),
        initial,
        bits: 64,
        signed: true,
    }
}

/// Body entries of a `signed_scan` that really runs with a possibly negative `n`.
fn signed_entries(program: &Program, n: i64) -> u64 {
    let mut values = [n, 0];
    let mut position = program.entry;
    let mut entries = 0;
    loop {
        let current = &program.nodes[position as usize];
        match current.action {
            Action::Assign {
                variable,
                value: Value::Const(value),
            } => values[variable as usize] = value as i64,
            Action::Increment { variable } => values[variable as usize] += 1,
            _ => {}
        }
        position = match &current.flow {
            Flow::Goto(next) => *next,
            Flow::Return => return entries,
            Flow::Branch { then_node, .. } if values[1] < values[0] => {
                entries += 1;
                *then_node
            }
            Flow::Branch { else_node, .. } => *else_node,
        };
    }
}

/// A bound's value when its only input is the possibly negative `n`.
fn signed_value(bound: &Bound, n: i64) -> i64 {
    match bound {
        Bound::Const(value) => *value as i64,
        Bound::Input(_) => n,
        Bound::Max(items) => items
            .iter()
            .map(|item| signed_value(item, n))
            .max()
            .unwrap(),
        other => panic!("unexpected {other}"),
    }
}

fn opaque_reason(program: &Program) -> String {
    let Outcome::Partial(result) = analyze(program) else {
        panic!("{:?}", analyze(program));
    };
    result.loops[0].opaque.as_ref().unwrap().reason.clone()
}

#[test]
fn signed_limit_is_floored_and_never_understates_the_entries() {
    let program = signed_scan(Value::Const(0));
    let bound = proven(&program).loop_body_entries;
    assert_eq!(bound.to_string(), "max(n, 0)");
    for n in -5..=5 {
        let entries = signed_entries(&program, n);
        assert_eq!(signed_value(&bound, n), entries as i64, "n = {n}");
    }
    assert_eq!(signed_entries(&program, -3), 0);
}

#[test]
fn unbounded_arithmetic_bounds_a_counter_as_a_signed_one() {
    let mut program = signed_scan(Value::Const(0));
    program.arithmetic = Arithmetic::Unbounded;
    let bound = proven(&program).loop_body_entries;
    assert_eq!(bound.to_string(), "max(n, 0)");
    for n in -5..=5 {
        assert_eq!(
            signed_value(&bound, n),
            signed_entries(&program, n) as i64,
            "n = {n}"
        );
    }
}

#[test]
fn unbounded_arithmetic_leaves_a_limit_past_the_modeled_width_unknown() {
    let mut program = signed_scan(Value::Const(0));
    program.arithmetic = Arithmetic::Unbounded;
    program.nodes[1] = node(
        Action::Nop,
        Flow::Branch {
            condition: Condition::Less {
                variable: 1,
                limit: Value::Const(1 << 63),
            },
            then_node: 2,
            else_node: 3,
        },
    );
    assert!(
        !matches!(analyze(&program), Outcome::Proven(_)),
        "{:?}",
        analyze(&program)
    );
}

#[test]
fn signed_constant_start_above_zero_keeps_an_upper_bound() {
    let program = signed_scan(Value::Const(2));
    let bound = proven(&program).loop_body_entries;
    for n in -5..=5 {
        assert!(signed_entries(&program, n) as i64 <= signed_value(&bound, n));
    }
}

#[test]
fn signed_start_that_may_be_negative_is_never_bounded_by_the_limit() {
    const BELOW_ZERO: &str = "Signed counter may start below zero";
    // `-5` lowers to an unknown value, since a constant is never negative.
    assert_eq!(opaque_reason(&signed_scan(Value::Unknown)), BELOW_ZERO);
    // A signed input may be negative, directly or through a local.
    assert_eq!(opaque_reason(&signed_scan(Value::Variable(0))), BELOW_ZERO);
    let input = signed_scan(Value::Input("n".into()));
    assert_eq!(opaque_reason(&input), BELOW_ZERO);

    // A counter that is a parameter starts wherever the caller says.
    let mut parameter = signed_scan(Value::Const(0));
    parameter.variables[1].initial = Value::Input("i".into());
    parameter.inputs.push("i".into());
    parameter.nodes[0].action = Action::Nop;
    assert_eq!(opaque_reason(&parameter), BELOW_ZERO);

    // A constant on one path leaves the parameter's value on the other.
    let mut conditional = signed_scan(Value::Const(0));
    conditional.variables[1].initial = Value::Input("i".into());
    conditional.inputs.push("i".into());
    conditional.nodes.push(conditional.nodes[0].clone());
    conditional.nodes[0] = node(
        Action::Nop,
        Flow::Branch {
            condition: Condition::Unknown,
            then_node: 4,
            else_node: 1,
        },
    );
    assert_eq!(opaque_reason(&conditional), BELOW_ZERO);

    // An unknown reset before the guard also reaches it.
    let mut reset = signed_scan(Value::Const(0));
    reset.nodes[0].flow = Flow::Goto(4);
    reset.nodes.push(node(
        Action::Assign {
            variable: 1,
            value: Value::Unknown,
        },
        Flow::Goto(1),
    ));
    assert_eq!(opaque_reason(&reset), BELOW_ZERO);
}

#[test]
fn a_limit_of_the_other_sign_stays_unproven() {
    const OTHER_TYPE: &str = "Limit does not fit the captured counter type";
    let mut signed_counter = signed_scan(Value::Const(0));
    signed_counter.variables[0].signed = false;
    assert_eq!(opaque_reason(&signed_counter), OTHER_TYPE);

    let mut unsigned_counter = signed_scan(Value::Const(0));
    unsigned_counter.variables[1].signed = false;
    assert_eq!(opaque_reason(&unsigned_counter), OTHER_TYPE);
}

#[test]
fn a_signed_type_holds_only_half_of_its_unsigned_constants() {
    let limited = |limit: u64| {
        let mut program = signed_scan(Value::Const(0));
        program.variables[1].bits = 8;
        program.nodes[1].flow = Flow::Branch {
            condition: Condition::Less {
                variable: 1,
                limit: Value::Const(limit),
            },
            then_node: 2,
            else_node: 3,
        };
        program
    };
    assert_eq!(proven(&limited(127)).loop_body_entries, Bound::Const(127));
    assert_eq!(
        opaque_reason(&limited(128)),
        "Unknown, cyclic or out-of-range input value"
    );
}

#[test]
fn writing_a_signed_local_outside_a_counter_does_not_end_the_analysis() {
    let mut program = signed_scan(Value::Variable(0));
    program.nodes[1] = increment(1, 3);
    program.loop_headers.clear();
    assert_eq!(proven(&program).loop_body_entries, Bound::Const(0));
}
