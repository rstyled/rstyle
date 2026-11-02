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
            },
            Variable {
                name: "m".into(),
                initial: Value::Input("m".into()),
                bits: 32,
            },
            Variable {
                name: "i".into(),
                initial: Value::Unknown,
                bits: 32,
            },
            Variable {
                name: "j".into(),
                initial: Value::Unknown,
                bits: 32,
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
    match bound {
        Bound::Const(c) => *c,
        Bound::Input(name) => {
            if name == "n" {
                n
            } else {
                m
            }
        }
        Bound::Add(v) => v.iter().map(|b| bound_value(b, n, m)).sum(),
        Bound::Mul(v) => v.iter().map(|b| bound_value(b, n, m)).product(),
        Bound::Max(v) => v.iter().map(|b| bound_value(b, n, m)).max().unwrap_or(0),
    }
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
    assert!(matches!(analyze(&skipped), Outcome::Unknown { .. }));
    let mut limit = scan();
    limit.nodes[2].action = Action::Assign {
        variable: 0,
        value: Value::Const(0),
    };
    assert!(matches!(analyze(&limit), Outcome::Unknown { .. }));
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
        assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
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
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
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
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
}

#[test]
fn repeated_outer_increment_inside_inner_cycle_is_not_a_guarded_scan() {
    let mut program = nested(false);
    program.nodes[5] = increment(2, 9);
    program.nodes[7] = node(Action::Nop, Flow::Goto(2));
    program.nodes.push(increment(3, 4));
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
}

#[test]
fn saved_limit_never_reads_a_later_assignment_as_its_entry_snapshot() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "saved".into(),
        initial: Value::Unknown,
        bits: 32,
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
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
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
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
}

#[test]
fn initializer_assignments_can_supply_an_immutable_limit() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "length".into(),
        initial: Value::Unknown,
        bits: 32,
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
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));
}

#[test]
fn unrelated_accumulator_increments_do_not_change_private_scan_bounds() {
    let mut program = scan();
    program.variables.push(Variable {
        name: "total".into(),
        initial: Value::Const(0),
        bits: 8,
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
    assert!(matches!(analyze(&writes_limit), Outcome::Unknown { .. }));

    let mut persistent = callback_scan(true);
    persistent.nodes[2].action = Action::Nop;
    persistent.nodes[0] = assign(2, 10);
    persistent.nodes.push(assign(3, 1));
    assert!(matches!(analyze(&persistent), Outcome::Unknown { .. }));

    let mut modifying = callback_scan(false);
    modifying.nodes[5].action = opaque_call("score", true);
    assert!(matches!(analyze(&modifying), Outcome::Unknown { .. }));
}

fn assume() -> Limits {
    Limits {
        opaque_loops: true,
        ..Limits::default()
    }
}

fn assumed(program: &Program) -> Assessment {
    match analyze_with_limits(program, assume()) {
        Outcome::Proven(result) => result,
        other => panic!("{other:?}"),
    }
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
fn assume_mode_bounds_unprovable_loops_by_an_opaque_factor() {
    let program = opaque_outer();
    assert!(matches!(analyze(&program), Outcome::Unknown { .. }));

    let result = assumed(&program);
    let outer = Bound::Input("?L2".into());
    assert_eq!(result.loops[0].body_entries, outer);
    assert!(result.loops[0].opaque.is_some());
    // The inner scan still resets per outer entry, so it multiplies the opaque factor.
    assert_eq!(
        result.loops[1].body_entries,
        Bound::Mul(vec![outer, Bound::Input("m".into())])
    );
    assert_eq!(result.loops[1].opaque, None);
}

#[test]
fn assume_mode_keeps_proven_siblings_and_whole_owner_failures() {
    let mut twice = scan();
    twice.nodes[2].action = Action::Increment { variable: 2 };
    let result = assumed(&twice);
    assert!(result.loops[0].opaque.is_some());
    assert_eq!(result.loops[0].body_entries, Bound::Input("?L1".into()));

    let sequential = assumed(&nested(false));
    assert_eq!(sequential, proven(&nested(false)));

    let mut unsupported = opaque_outer();
    unsupported.nodes[5].action = Action::Unsupported {
        reason: "unmodeled".into(),
    };
    assert!(matches!(
        analyze_with_limits(&unsupported, assume()),
        Outcome::Unknown { .. }
    ));
}

#[test]
fn assume_mode_marks_changed_limits_stray_increments_and_nested_opaque_loops() {
    let mut changed = scan();
    changed.nodes[2].action = Action::Assign {
        variable: 0,
        value: Value::Const(5),
    };
    let result = assumed(&changed);
    assert!(result.loops[0].opaque.is_some());

    // The counter is incremented again after the loop exits, outside its guard.
    let mut stray = scan();
    stray.nodes[4] = increment(2, 5);
    stray.nodes.push(node(Action::Nop, Flow::Return));
    let result = assumed(&stray);
    assert!(result.loops.iter().any(|item| {
        item.opaque
            .as_deref()
            .is_some_and(|reason| reason.contains("no-overflow"))
    }));

    let mut inner = nested(true);
    inner.nodes[4].flow = Flow::Branch {
        condition: Condition::Unknown,
        then_node: 5,
        else_node: 7,
    };
    let result = assumed(&inner);
    assert_eq!(result.loops[0].body_entries, Bound::Input("n".into()));
    assert_eq!(
        result.loops[1].body_entries,
        Bound::Mul(vec![Bound::Input("n".into()), Bound::Input("?L4".into())])
    );
}

#[test]
fn assume_mode_ignores_declared_loops_that_never_cycle() {
    let mut exits = scan();
    // The body returns instead of reaching its increment, so the guard never repeats.
    exits.nodes[2].flow = Flow::Goto(4);
    assert!(matches!(analyze(&exits), Outcome::Unknown { .. }));
    let result = assumed(&exits);
    assert!(result.loops.is_empty());
    assert_eq!(result.calls[0].count, Bound::Const(1));
}
