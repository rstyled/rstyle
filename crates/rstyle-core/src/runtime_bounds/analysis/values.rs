use super::*;

pub(super) struct Values<'a> {
    program: &'a Program,
    graph: &'a Graph,
    point: usize,
    /// The type a resolved value has to fit, when it is compared against a counter.
    within: Option<Integer>,
}

impl<'a> Values<'a> {
    pub(super) fn new(
        program: &'a Program,
        graph: &'a Graph,
        point: usize,
        within: Option<Integer>,
    ) -> Self {
        Self {
            program,
            graph,
            point,
            within,
        }
    }

    pub(super) fn bound(&self, value: &Value) -> Checked<Bound> {
        self.resolve(value, &mut BTreeSet::new())
    }

    fn resolve(&self, value: &Value, seen: &mut BTreeSet<u32>) -> Checked<Bound> {
        match value {
            Value::Const(n) if self.within.is_none_or(|within| within.holds(*n)) => {
                Ok(Bound::Const(*n))
            }
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
        if !self.fits(variable.integer()) {
            return Err(unknown("Limit does not fit the captured counter type"));
        }
        let (initial, point) = self.initial(id)?;
        let snapshot = Self { point, ..*self };

        snapshot.resolve(initial, seen)
    }

    fn input(&self, name: &str) -> Checked<Bound> {
        let integers: BTreeSet<_> = self
            .program
            .variables
            .iter()
            .filter(|variable| variable.initial == Value::Input(name.into()))
            .map(Variable::integer)
            .collect();
        let integer = integers
            .first()
            .filter(|integer| integers.len() == 1 && self.fits(**integer))
            .ok_or_else(|| unknown("Input lacks one consistent captured integer type"))?;
        let input = Bound::Input(name.into());

        Ok(if integer.signed {
            Bound::floored(input)
        } else {
            input
        })
    }

    fn fits(&self, integer: Integer) -> bool {
        self.within.is_none_or(|within| within.contains(integer))
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
