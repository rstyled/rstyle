use super::*;

pub(super) struct Values<'a> {
    program: &'a Program,
    graph: &'a Graph,
    point: usize,
    bits: u8,
}

impl<'a> Values<'a> {
    pub(super) fn new(program: &'a Program, graph: &'a Graph, point: usize, bits: u8) -> Self {
        Self {
            program,
            graph,
            point,
            bits,
        }
    }

    pub(super) fn bound(&self, value: &Value) -> Checked<Bound> {
        self.resolve(value, &mut BTreeSet::new())
    }

    fn resolve(&self, value: &Value, seen: &mut BTreeSet<u32>) -> Checked<Bound> {
        match value {
            Value::Const(n) if self.bits == 64 || *n < (1u64 << self.bits) => Ok(Bound::Const(*n)),
            Value::Input(name) if self.program.inputs.contains(name) => self.input(name),
            Value::Variable(id) if seen.insert(*id) => self.variable(*id, seen),
            _ => Err(unknown("Unknown, cyclic or out-of-range input value")),
        }
    }

    fn variable(&self, id: u32, seen: &mut BTreeSet<u32>) -> Checked<Bound> {
        let variable = self
            .program
            .variables
            .get(id as usize)
            .ok_or_else(|| unknown("Missing variable"))?;
        if variable.bits > self.bits {
            return Err(unknown("Limit exceeds the captured counter width"));
        }
        let (initial, point) = self.initial(id)?;
        let snapshot = Self { point, ..*self };

        snapshot.resolve(initial, seen)
    }

    fn input(&self, name: &str) -> Checked<Bound> {
        let widths: BTreeSet<_> = self
            .program
            .variables
            .iter()
            .filter(|variable| variable.initial == Value::Input(name.into()))
            .map(|variable| variable.bits)
            .collect();
        if widths.len() != 1 || widths.first().is_none_or(|width| *width > self.bits) {
            return Err(unknown(
                "Input lacks one consistent captured unsigned width",
            ));
        }

        Ok(Bound::Input(name.into()))
    }

    fn initial(&self, variable: u32) -> Checked<(&'a Value, usize)> {
        if self.program.nodes.iter().enumerate().any(|(i, node)| {
            self.graph.reachable[i] && node.action == Action::Increment { variable }
        }) {
            return Err(unknown("Snapshot value depends on an incremented variable"));
        }
        let writes: Vec<_> = self
            .program
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                if !self.graph.reachable[i] {
                    return None;
                }
                match &node.action {
                    Action::Assign {
                        variable: target,
                        value,
                    } if *target == variable => Some((i, value)),
                    _ => None,
                }
            })
            .collect();
        if writes.is_empty() {
            return Ok((
                &self.program.variables[variable as usize].initial,
                self.program.entry as usize,
            ));
        }

        let first = writes[0].1;
        if writes
            .iter()
            .any(|(i, value)| *value != first || !self.graph.dominators[self.point][*i])
        {
            return Err(unknown(
                "Limit lacks an invariant dominating initialization",
            ));
        }

        Ok((first, writes[0].0))
    }
}

pub(super) fn references(value: &Value, variable: u32, program: &Program) -> bool {
    let mut next = value;
    let mut seen = BTreeSet::new();
    while let Value::Variable(id) = next {
        if *id == variable {
            return true;
        }
        if !seen.insert(*id) {
            return true;
        }
        let Some(item) = program.variables.get(*id as usize) else {
            return true;
        };
        next = &item.initial;
    }
    false
}
