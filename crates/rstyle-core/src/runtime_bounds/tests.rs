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
            node(
                Action::Call {
                    callee: "visit".into(),
                    may_modify: false,
                },
                Flow::Goto(3),
            ),
            increment(2, 1),
            node(Action::Nop, Flow::Return),
        ],
        entry: 0,
        loop_headers: vec![1],
        arithmetic: Arithmetic::Checked,
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
    alias.nodes[2].action = Action::Call {
        callee: "unknown".into(),
        may_modify: true,
    };
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
