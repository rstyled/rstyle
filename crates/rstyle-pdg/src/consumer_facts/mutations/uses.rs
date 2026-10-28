use super::{
    Availability, Boundary, BoundaryAvailability, Coverage, Execution, Root, UseFormal,
    UseInventory, UseOccurrence, UseRole,
};
use crate::{ByteSpan, HirNode};
use std::collections::{BTreeMap, BTreeSet};

const MAX_USES: usize = 65536;

pub(super) struct Scan {
    inventory: UseInventory,
    outcome: Outcome,
    seen: BTreeSet<(u32, u32, u32)>,
    declarations: BTreeMap<u32, (u32, u32, u32)>,
    declaration_ids: BTreeSet<(u32, u32, u32)>,
}

impl Scan {
    pub fn new() -> Self {
        Self {
            inventory: UseInventory {
                availability: Availability::Known(()),
                coverage: Coverage::Complete,
                ..UseInventory::unrequested()
            },
            outcome: Outcome {
                availability: Availability::Known(()),
                coverage: Coverage::Complete,
                execution: Execution::Completed,
            },
            seen: BTreeSet::new(),
            declarations: BTreeMap::new(),
            declaration_ids: BTreeSet::new(),
        }
    }

    pub fn formal(&mut self, root: &Root, mutable: bool) -> Result<(), String> {
        let Some(ordinal) = root.formal else {
            return Ok(());
        };
        let identity = (root.native.krate, root.native.owner, root.native.local);
        if self.declarations.contains_key(&ordinal) || self.declaration_ids.contains(&identity) {
            return Err("Formal use declaration inventory repeats an original binding".into());
        }

        if self.reserve(root.declaration_span) {
            self.retain_declaration(ordinal, identity);
            self.inventory.formals.push(UseFormal {
                ordinal,
                span: root.declaration_span,
                native: root.native.clone(),
                mutable,
            });
        }
        Ok(())
    }

    fn retain_declaration(&mut self, ordinal: u32, identity: (u32, u32, u32)) {
        self.declarations.insert(ordinal, identity);
        self.declaration_ids.insert(identity);
    }

    pub fn occurrence(&mut self, node: &HirNode, root: &Root, role: UseRole) -> Result<(), String> {
        let Some(formal) = root.formal else {
            return Ok(());
        };
        let declaration = (root.native.krate, root.native.owner, root.native.local);
        if self.declarations.get(&formal) != Some(&declaration) {
            self.boundary(
                node.span,
                BoundaryAvailability::Unknown,
                "Formal use has no checked owned-u32 declaration",
            );
            return Ok(());
        }

        if !self.reserve(node.span) {
            return Ok(());
        }
        self.retain_occurrence(node, formal, role)
    }

    fn retain_occurrence(
        &mut self,
        node: &HirNode,
        formal: u32,
        role: UseRole,
    ) -> Result<(), String> {
        if !self
            .seen
            .insert((node.hir.krate, node.hir.owner, node.hir.local))
        {
            return Err("Formal use inventory repeats an original native occurrence".into());
        }
        let occurrence = UseOccurrence {
            native: node.hir.clone(),
            span: node.span,
            formal,
            role,
        };
        self.inventory.occurrences.push(occurrence);
        Ok(())
    }

    pub fn boundary(&mut self, span: ByteSpan, availability: BoundaryAvailability, reason: &str) {
        let availability_fact = match availability {
            BoundaryAvailability::Unknown => Availability::Unknown {
                reason: reason.into(),
            },
            BoundaryAvailability::Unsupported => Availability::Unsupported {
                reason: reason.into(),
            },
        };
        let outcome = Outcome::incomplete(availability_fact, self.outcome.execution);
        self.outcome = outcome;
        self.retain_boundary(span, availability, reason);
    }

    fn retain_boundary(
        &mut self,
        span: ByteSpan,
        availability: BoundaryAvailability,
        reason: &str,
    ) {
        if self.reserve(span) {
            self.inventory.boundaries.push(Boundary {
                span,
                availability,
                reason: reason.into(),
            });
        }
    }

    pub fn exhausted(&mut self, span: ByteSpan) {
        let outcome = Outcome::incomplete(
            Availability::Unknown {
                reason: "Formal use observation budget exhausted".into(),
            },
            Execution::BudgetExhausted,
        );
        let boundary = (self.inventory.boundaries.is_empty()
            && self.inventory.formals.len() + self.inventory.occurrences.len() < MAX_USES)
            .then(|| Boundary {
                span,
                availability: BoundaryAvailability::Unknown,
                reason: "Formal use observation budget exhausted".into(),
            });

        self.outcome = outcome;
        if let Some(boundary) = boundary {
            self.inventory.boundaries.push(boundary);
        }
    }

    fn reserve(&mut self, span: ByteSpan) -> bool {
        if self.inventory.formals.len()
            + self.inventory.occurrences.len()
            + self.inventory.boundaries.len()
            >= MAX_USES
        {
            self.exhausted(span);
            return false;
        }
        true
    }

    pub fn finish(self, expected: usize, signature: &Availability<()>) -> UseInventory {
        let outcome = self
            .outcome
            .finished(self.inventory.formals.len() == expected, signature);
        UseInventory {
            availability: outcome.availability,
            coverage: outcome.coverage,
            execution: outcome.execution,
            ..self.inventory
        }
    }
}

struct Outcome {
    availability: Availability<()>,
    coverage: Coverage,
    execution: Execution,
}

impl Outcome {
    fn incomplete(availability: Availability<()>, execution: Execution) -> Self {
        Self {
            availability,
            coverage: Coverage::Incomplete,
            execution,
        }
    }

    fn finished(self, declarations_complete: bool, signature: &Availability<()>) -> Self {
        if !declarations_complete {
            Self::incomplete(
                Availability::Unknown {
                    reason: "Complete owned-u32 formal declaration inventory is unavailable".into(),
                },
                self.execution,
            )
        } else if !matches!(signature, Availability::Known(())) {
            Self::incomplete(signature.clone(), self.execution)
        } else {
            self
        }
    }
}
