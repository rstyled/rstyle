use super::*;
use std::collections::BTreeMap;

/// Never a captured variable; opaque loops have no modeled counter.
const NO_COUNTER: u32 = u32::MAX;

pub(super) fn discover(program: &Program, graph: &Graph) -> Checked<Vec<Loop>> {
    let mut regions: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (source, targets) in graph.successors.iter().enumerate() {
        if !graph.reachable[source] {
            continue;
        }
        for &target in targets {
            if graph.dominators[source][target] {
                regions
                    .entry(target)
                    .or_default()
                    .extend(region(graph, source, target));
            }
        }
    }

    acyclic_without_backedges(graph)?;
    let declared: BTreeSet<_> = program.loop_headers.iter().map(|&h| h as usize).collect();
    let observed: BTreeSet<_> = regions.keys().copied().collect();
    // A declared loop that never cycles (its body always exits) is not a loop; a
    // cycle without a declared header is unmodeled control flow.
    if declared.len() != program.loop_headers.len() || !observed.is_subset(&declared) {
        return Err(unknown(
            "Captured loops include an acyclic, missing or unsupported loop guard",
        ));
    }

    // A reachable declared header with no backedge runs its body at most once per
    // entry of its innermost enclosing cycle.
    let acyclic: Vec<_> = declared
        .iter()
        .filter(|header| graph.reachable[**header] && !observed.contains(header))
        .map(|&header| once(header))
        .collect();

    regions
        .into_iter()
        .map(
            |(header, members)| match describe(program, graph, header, members.clone()) {
                Err(Failure::Unknown(reason)) => Ok(Loop {
                    header,
                    members,
                    counter: NO_COUNTER,
                    limit: Value::Unknown,
                    increment: header,
                    opaque: Some(reason),
                    once: false,
                }),
                other => other,
            },
        )
        .chain(acyclic.into_iter().map(Ok))
        .collect()
}

fn once(header: usize) -> Loop {
    Loop {
        header,
        members: BTreeSet::from([header]),
        counter: NO_COUNTER,
        limit: Value::Unknown,
        increment: header,
        opaque: None,
        once: true,
    }
}

// A failure confined to one loop makes only that loop opaque.
fn per_loop(result: Checked<()>, item: &mut Loop) -> Checked<()> {
    match result {
        Err(Failure::Unknown(reason)) => {
            item.opaque.get_or_insert(reason);
            Ok(())
        }
        other => other,
    }
}

fn region(graph: &Graph, source: usize, header: usize) -> BTreeSet<usize> {
    let mut members = BTreeSet::from([header]);

    let mut pending = vec![source];
    while let Some(index) = pending.pop() {
        if !members.insert(index) {
            continue;
        }
        pending.extend(
            graph.predecessors[index]
                .iter()
                .copied()
                .filter(|&i| graph.reachable[i]),
        );
    }
    members
}

fn acyclic_without_backedges(graph: &Graph) -> Checked<()> {
    let mut incoming = vec![0; graph.successors.len()];
    for (source, targets) in graph.successors.iter().enumerate() {
        if graph.reachable[source] {
            for &target in targets {
                if !graph.dominators[source][target] {
                    incoming[target] += 1;
                }
            }
        }
    }
    let mut pending: Vec<_> = incoming
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| (graph.reachable[i] && n == 0).then_some(i))
        .collect();
    let mut visited = 0;
    while let Some(source) = pending.pop() {
        visited += 1;
        for &target in &graph.successors[source] {
            if graph.dominators[source][target] {
                continue;
            }
            incoming[target] -= 1;
            if incoming[target] == 0 {
                pending.push(target);
            }
        }
    }
    if visited != graph.reachable.iter().filter(|&&r| r).count() {
        return Err(unknown("Irreducible or uncovered cyclic control flow"));
    }

    Ok(())
}

fn describe(
    program: &Program,
    graph: &Graph,
    header: usize,
    members: BTreeSet<usize>,
) -> Checked<Loop> {
    let node = &program.nodes[header];
    let Flow::Branch {
        condition: Condition::Less { variable, limit },
        then_node,
        else_node,
    } = &node.flow
    else {
        return Err(unknown("Loop requires an unsigned less-than guard"));
    };
    if node.action != Action::Nop
        || !members.contains(&(*then_node as usize))
        || members.contains(&(*else_node as usize))
    {
        return Err(unknown(
            "Loop entry/exit guard is not a supported increasing scan",
        ));
    }

    let increment = unique_increment(program, &members, *variable)?;
    if !graph.dominators[increment][*then_node as usize] {
        return Err(unknown("Increment is not guarded by loop entry"));
    }
    for &source in &graph.predecessors[header] {
        if members.contains(&source) && !graph.dominators[source][increment] {
            return Err(unknown("A backedge can skip the counter increment"));
        }
    }

    Ok(Loop {
        header,
        members,
        counter: *variable,
        limit: limit.clone(),
        increment,
        opaque: None,
        once: false,
    })
}

fn unique_increment(program: &Program, members: &BTreeSet<usize>, variable: u32) -> Checked<usize> {
    let increments: Vec<_> = members
        .iter()
        .copied()
        .filter(|&i| program.nodes[i].action == Action::Increment { variable })
        .collect();
    let [increment] = increments.as_slice() else {
        return Err(unknown("Loop needs exactly one explicit unit increment"));
    };

    Ok(*increment)
}

pub(super) fn validate(program: &Program, graph: &Graph, loops: &mut [Loop]) -> Checked<()> {
    structure(loops)?;
    for (index, node) in program
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| graph.reachable[*i])
    {
        match &node.action {
            Action::Unsupported { reason } => return Err(unknown(reason)),
            Action::Call {
                may_modify: true, ..
            } => return Err(unknown("Call may modify a bound-relevant value or alias")),
            Action::Increment { variable }
                if loops.iter().any(|item| item.counter == *variable)
                    && !loops
                        .iter()
                        .any(|item| item.counter == *variable && item.increment == index) =>
            {
                let reason = "Increment lacks a current-state no-overflow guard";
                for item in loops.iter_mut().filter(|item| item.counter == *variable) {
                    item.opaque.get_or_insert(reason.into());
                }
            }
            Action::Assign { variable, .. } | Action::Increment { variable }
                if *variable as usize >= program.variables.len() =>
            {
                return Err(unknown("Unknown assigned variable"));
            }
            _ => {}
        }
    }

    for item in loops.iter_mut() {
        if item.opaque.is_some() || item.once {
            continue;
        }
        let result = validate_loop(program, graph, item);
        per_loop(result, item)?;
    }

    Ok(())
}

// Composition multiplies along a nesting tree of loops with distinct counters.
fn structure(loops: &[Loop]) -> Checked<()> {
    for item in loops {
        if item.counter != NO_COUNTER
            && loops
                .iter()
                .any(|other| other.header != item.header && other.counter == item.counter)
        {
            return Err(unknown("A counter is shared between distinct loop guards"));
        }
        if loops.iter().any(|other| {
            !item.members.is_disjoint(&other.members)
                && !item.members.is_subset(&other.members)
                && !other.members.is_subset(&item.members)
        }) {
            return Err(unknown("Overlapping loop regions cannot be composed"));
        }
    }
    Ok(())
}

fn validate_loop(program: &Program, graph: &Graph, item: &Loop) -> Checked<()> {
    let variable = program
        .variables
        .get(item.counter as usize)
        .ok_or_else(|| unknown("Unknown loop counter"))?;
    if program.arithmetic == Arithmetic::Unknown {
        return Err(unknown("Missing integer arithmetic semantics"));
    }
    values::Values::new(program, graph, item.header, variable.bits).bound(&item.limit)?;
    for (index, node) in program
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| graph.reachable[*i])
    {
        if let Some(written) = written(&node.action) {
            if values::references(&item.limit, written, program)
                && (item.members.contains(&index)
                    || !graph.dominators[item.header][index]
                    || matches!(node.action, Action::Increment { .. }))
            {
                return Err(unknown("Loop limit changes after entry"));
            }
            if written == item.counter && index != item.increment {
                validate_reset(program, graph, item, index)?;
            }
        }
    }

    Ok(())
}

fn written(action: &Action) -> Option<u32> {
    match action {
        Action::Assign { variable, .. } | Action::Increment { variable } => Some(*variable),
        _ => None,
    }
}

fn validate_reset(program: &Program, graph: &Graph, item: &Loop, index: usize) -> Checked<()> {
    let Action::Assign { value, .. } = &program.nodes[index].action else {
        return Err(unknown("Unsupported counter modification"));
    };
    if item.members.contains(&index) {
        return Err(unknown("Counter resets inside its own cycle"));
    }
    values::Values::new(
        program,
        graph,
        index,
        program.variables[item.counter as usize].bits,
    )
    .bound(value)?;

    Ok(())
}

pub(super) fn bounds(
    program: &Program,
    graph: &Graph,
    loops: &mut [Loop],
    limits: Limits,
) -> Checked<Vec<Bound>> {
    let mut result = vec![None; loops.len()];
    let mut order: Vec<_> = (0..loops.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(loops[i].members.len()));

    for index in order {
        let context = ResetBounds {
            program,
            graph,
            loops,
            bounds: &result,
        };
        let modeled = match &loops[index].opaque {
            None if loops[index].once => Ok(enclosing(loops, &result, index)),
            None => context.modeled(&loops[index]),
            Some(_) => Err(unknown("opaque")),
        };
        let bound = match modeled {
            Ok(bound) => bound,
            Err(Failure::Unknown(reason)) => {
                loops[index].opaque.get_or_insert(reason);
                opaque_bound(program, loops, &result, index)
            }
            Err(failure) => return Err(failure),
        };
        if bound.size() > limits.expression_nodes {
            return Err(Failure::Budget("Symbolic bound budget exceeded".into()));
        }
        result[index] = Some(bound);
    }

    Ok(result.into_iter().map(Option::unwrap).collect())
}

// An opaque loop runs an unknown number of times per entry of its innermost parent.
fn opaque_bound(
    program: &Program,
    loops: &[Loop],
    result: &[Option<Bound>],
    index: usize,
) -> Bound {
    Bound::mul(
        enclosing(loops, result, index),
        Bound::opaque(OpaqueKind::Loop, site(program, loops[index].header)),
    )
}

/// An opaque loop's site: its source location and header node, unique per owner.
pub(super) fn site(program: &Program, header: usize) -> String {
    program.nodes[header].source.as_ref().map_or_else(
        || format!("node {header}"),
        |source| format!("{}:{}:{}#{header}", source.file, source.line, source.column),
    )
}

// Body entries of the innermost loop strictly enclosing `index`, or one invocation.
fn enclosing(loops: &[Loop], result: &[Option<Bound>], index: usize) -> Bound {
    let item = &loops[index];
    loops
        .iter()
        .enumerate()
        .filter(|(_, other)| other.header != item.header && item.members.is_subset(&other.members))
        .min_by_key(|(_, other)| other.members.len())
        .and_then(|(parent, _)| result[parent].clone())
        .unwrap_or(Bound::Const(1))
}

struct ResetBounds<'a> {
    program: &'a Program,
    graph: &'a Graph,
    loops: &'a [Loop],
    bounds: &'a [Option<Bound>],
}

impl ResetBounds<'_> {
    fn modeled(&self, item: &Loop) -> Checked<Bound> {
        let bits = self.program.variables[item.counter as usize].bits;
        let at_header = values::Values::new(self.program, self.graph, item.header, bits);
        let local = at_header.bound(&item.limit)?;

        Ok(Bound::mul(self.count(item)?, local))
    }

    fn count(&self, item: &Loop) -> Checked<Bound> {
        let mut resets = Vec::new();
        for (index, node) in self
            .program
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| self.graph.reachable[*i])
        {
            if !matches!(node.action, Action::Assign { variable, .. } if variable == item.counter) {
                continue;
            }
            if let Some(bound) = self.reset(item, index)? {
                resets.push(bound);
            }
        }

        match resets.len() {
            0 => self.persistent(item),
            1 => Ok(resets.remove(0)),
            _ => Err(unknown(
                "Multiple dynamic resets require additional amortized accounting",
            )),
        }
    }

    fn reset(&self, item: &Loop, index: usize) -> Checked<Option<Bound>> {
        let parent = self
            .loops
            .iter()
            .enumerate()
            .filter(|(_, parent)| {
                parent.header != item.header
                    && item.members.is_subset(&parent.members)
                    && parent.members.contains(&index)
            })
            .min_by_key(|(_, parent)| parent.members.len());
        let Some((parent_index, parent)) = parent else {
            return Ok(None);
        };
        if !self.graph.dominators[item.header][index] {
            return Err(unknown(
                "Conditional reset is not proved once per enclosing entry",
            ));
        }
        if self
            .loops
            .iter()
            .any(|nested| nested.header != parent.header && nested.members.contains(&index))
        {
            return Err(unknown("Reset is repeated by another nested cycle"));
        }
        // Between resets the scan is a persistent cursor over the intermediate loops'
        // re-entries, so an early exit can enter the body without advancing it.
        let intermediate = self.loops.iter().any(|middle| {
            middle.header != item.header
                && middle.header != parent.header
                && item.members.is_subset(&middle.members)
                && middle.members.is_subset(&parent.members)
        });
        if intermediate && self.early_exit(item) {
            return Err(unknown(
                "Reset scan re-entered by an intermediate loop has uncharged early exits",
            ));
        }

        Ok(Some(
            self.bounds[parent_index]
                .clone()
                .ok_or_else(|| unknown("Missing enclosing bound"))?,
        ))
    }

    fn persistent(&self, item: &Loop) -> Checked<Bound> {
        let nested = self
            .loops
            .iter()
            .any(|parent| parent.header != item.header && item.members.is_subset(&parent.members));
        if nested && self.early_exit(item) {
            return Err(unknown(
                "Persistent cursor has uncharged early-exit entries",
            ));
        }

        Ok(Bound::Const(1))
    }

    // Leaving the cycle anywhere but its guard enters the body without a full step.
    fn early_exit(&self, item: &Loop) -> bool {
        item.members.iter().any(|&source| {
            source != item.header
                && self.graph.successors[source]
                    .iter()
                    .any(|target| !item.members.contains(target))
        })
    }
}
