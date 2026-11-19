//! Native storage certificates from resolved allocation roles. They permit replacing
//! an inert element without assuming that an arbitrary subscript has no setter.

use super::super::model::{
    DefinitionKind, Dispatch, Node, NodeKind, PlaceMode, RefStatus, TargetKind, TypeKind,
};
use super::Collector;
use super::facts::Facts;
use super::writes::value_only;
use crate::advisory::runtime_bounds::MAX_NODES;
use crate::runtime_bounds::contracts::{Allocation, Contracts, Storage};

impl<'a> Collector<'_, '_, 'a> {
    /// Runtime operands only. The call action still holds every original argument slot.
    pub(super) fn allocation_operands(
        &self,
        call: &'a Node,
    ) -> Result<Option<Vec<&'a Node>>, String> {
        let Some(role) = allocation(self.facts, call) else {
            return Ok(None);
        };
        let inputs = self.facts.body.children(call)?;
        let runtime = inputs.into_iter().skip(1).enumerate();
        let runtime = runtime.filter(|(index, _)| Some(*index as u32) != role.type_argument);
        Ok(Some(runtime.map(|(_, node)| node).collect()))
    }

    pub(super) fn native_element(&self, place: &Node) -> bool {
        indexed(self.facts, place).is_some()
            && self.hidden_call(place).is_ok_and(|reason| reason.is_none())
    }

    /// These two operands are certified effect-free, so evaluating them cannot reorder
    /// calls or writes relative to the RHS under a language's assignment-order policy.
    pub(super) fn assignment_place(&mut self, place: &Node, next: u32) -> Result<u32, String> {
        let Some((base, index)) = indexed(self.facts, place) else {
            return Ok(next);
        };
        let entry = self.expr(index, next)?;
        self.expr(base, entry)
    }
}

fn allocation(facts: &Facts<'_, '_>, call: &Node) -> Option<&'static Allocation> {
    let reference = facts.reference(call)?;
    if call.kind != NodeKind::Call
        || reference.status != RefStatus::Resolved
        || reference.dispatch != Some(Dispatch::Builtin)
        || reference.arguments.is_some()
    {
        return None;
    }
    let target = reference.target.as_ref()?;
    let definition = facts.declared(reference)?;
    if target.kind != TargetKind::Definition
        || !definition.standard
        || definition.kind != DefinitionKind::Function
    {
        return None;
    }
    let role = Contracts::embedded().allocation(definition.operation.as_deref()?)?;
    let ty = facts.ty(call)?;
    if role.storage != Storage::FreshNativeSequence
        || ty.kind != TypeKind::Sequence
        || ty.fixed != Some(false)
        || !value_only(facts, ty)
    {
        return None;
    }
    let children = facts.body.children(call).ok()?;
    let arity = usize::try_from(role.arity).ok()?;
    if children.len() != arity.checked_add(1)?
        || !named_allocator(facts, children[0], definition.id.as_str())
    {
        return None;
    }
    let valid = children[1..].iter().enumerate().all(|(index, operand)| {
        if Some(index as u32) == role.type_argument {
            static_type(operand, call)
        } else {
            facts.integer(facts.adjusted(operand)).is_some()
        }
    });
    valid.then_some(role)
}

fn named_allocator(facts: &Facts<'_, '_>, callee: &Node, definition: &str) -> bool {
    callee.kind == NodeKind::Identifier
        && callee.children.is_empty()
        && !facts.adjusted_of(callee)
        && facts
            .reference(callee)
            .is_some_and(|reference| reference.status == RefStatus::Resolved)
        && facts
            .resolved(callee)
            .is_some_and(|(kind, named)| kind == TargetKind::Definition && named.id == definition)
        && facts.ty(callee).is_some_and(|ty| {
            ty.kind == TypeKind::Function && ty.definition.as_deref() == Some(definition)
        })
}

fn static_type(operand: &Node, call: &Node) -> bool {
    operand.kind == NodeKind::TypeAnnotation
        && operand.children.is_empty()
        && operand.adjustments.as_ref().is_none_or(Vec::is_empty)
        && operand.modifiers.is_empty()
        && operand.ty.is_some()
        && operand.ty == call.ty
}

fn indexed<'a>(facts: &Facts<'_, 'a>, place: &Node) -> Option<(&'a Node, &'a Node)> {
    if place.kind != NodeKind::IndexAccess
        || place.children.len() != 2
        || facts.adjusted_of(place)
        || facts.reference(place).is_some_and(|reference| {
            reference.status != RefStatus::Resolved || reference.dispatch != Some(Dispatch::Builtin)
        })
    {
        return None;
    }
    let base = facts.body.child(place, 0).ok()?;
    let index = facts.body.child(place, 1).ok()?;
    let ty = facts.ty(base)?;
    let complete = ty.kind == TypeKind::Sequence
        && ty.element.is_some()
        && ty.element.as_ref() == place.ty.as_ref()
        && value_only(facts, ty);
    if complete && pure_index(facts, index) && fresh_local(facts, base) {
        Some((base, index))
    } else {
        None
    }
}

fn pure_index(facts: &Facts<'_, '_>, index: &Node) -> bool {
    if !index.children.is_empty()
        || facts.adjusted_of(index)
        || facts.integer(facts.ty(index)).is_none()
    {
        return false;
    }
    match index.kind {
        NodeKind::Literal => index
            .value
            .as_ref()
            .is_some_and(|value| value.kind == "integer"),
        NodeKind::Identifier => {
            facts
                .reference(index)
                .is_some_and(|reference| reference.status == RefStatus::Resolved)
                && facts.local(index).is_some_and(|symbol| {
                    facts.body.symbol(symbol).is_ok_and(|symbol| {
                        matches!(symbol.kind.as_str(), "local" | "parameter")
                            && symbol.ty == index.ty
                    })
                })
        }
        _ => false,
    }
}

fn fresh_local(facts: &Facts<'_, '_>, base: &Node) -> bool {
    let function = facts.body.function;
    if function.nodes.len().saturating_add(function.places.len()) > MAX_NODES * 4 {
        return false;
    }
    let Some(symbol) = facts.local(base).and_then(|id| facts.body.symbol(id).ok()) else {
        return false;
    };
    if symbol.kind != "local"
        || symbol.mutable
        || symbol.ty != base.ty
        || !base.children.is_empty()
        || facts.adjusted_of(base)
        || facts
            .reference(base)
            .is_none_or(|reference| reference.status != RefStatus::Resolved)
    {
        return false;
    }
    let Some(pattern) = symbol
        .node
        .as_deref()
        .and_then(|id| facts.body.node(id).ok())
    else {
        return false;
    };
    if pattern.kind != NodeKind::PatternBinding
        || !pattern.children.is_empty()
        || pattern.binding_mode.as_deref().unwrap_or("value") != "value"
    {
        return false;
    }
    initialized(facts, pattern, base)
        && Private {
            facts,
            symbol: &symbol.id,
        }
        .holds()
}

fn initialized(facts: &Facts<'_, '_>, pattern: &Node, base: &Node) -> bool {
    // Body validation gives each child one parent, so a binding has at most one declaration.
    facts
        .body
        .function
        .nodes
        .iter()
        .find(|node| {
            node.kind == NodeKind::LocalDeclaration && node.children.first() == Some(&pattern.id)
        })
        .filter(|declaration| declaration.children.len() == 2)
        .and_then(|declaration| facts.body.child(declaration, 1).ok())
        .is_some_and(|initializer| {
            initializer.ty == base.ty && allocation(facts, initializer).is_some()
        })
}

struct Private<'f, 'b, 'a> {
    facts: &'f Facts<'b, 'a>,
    symbol: &'a str,
}

impl Private<'_, '_, '_> {
    fn holds(&self) -> bool {
        let function = self.facts.body.function;
        let capture_budget = function.nodes.iter().try_fold(MAX_NODES * 4, |left, node| {
            left.checked_sub(node.captures.as_ref().map_or(0, Vec::len))
        });
        if capture_budget.is_none() || function.nodes.iter().any(|node| self.exposes(node)) {
            return false;
        }
        function.places.iter().all(|place| {
            place.root.symbol.as_deref() != Some(self.symbol)
                || (place.mode == PlaceMode::Value && place.escapes == Some(false))
        })
    }

    fn exposes(&self, node: &Node) -> bool {
        let unrecorded = matches!(node.kind, NodeKind::Closure | NodeKind::ItemDeclaration)
            && node.captures.is_none();
        let capture = node
            .captures
            .iter()
            .flatten()
            .any(|capture| capture.symbol == self.symbol);
        let reference = node.kind == NodeKind::Reference || self.facts.adjusted_of(node);
        let addressed =
            reference && root(self.facts, node, 32).is_none_or(|root| root == self.symbol);
        unrecorded || capture || addressed || self.rebinds(node).unwrap_or(true)
    }

    fn rebinds(&self, node: &Node) -> Result<bool, String> {
        let binding = matches!(
            node.kind,
            NodeKind::Assignment
                | NodeKind::CompoundAssignment
                | NodeKind::PatternCondition
                | NodeKind::LocalDeclaration
                | NodeKind::For
                | NodeKind::MatchArm
        );
        if !binding {
            return Ok(false);
        }
        let target = self.facts.body.child(node, 0)?;
        let original = node.kind == NodeKind::LocalDeclaration
            && self
                .facts
                .body
                .symbol(self.symbol)
                .is_ok_and(|symbol| symbol.node.as_deref() == Some(target.id.as_str()));
        Ok(!original && writes_binding(self.facts, target, self.symbol)?)
    }
}

fn writes_binding(facts: &Facts<'_, '_>, target: &Node, symbol: &str) -> Result<bool, String> {
    let mut pending = vec![target];
    let mut work = 0;
    while let Some(part) = pending.pop() {
        work += 1;
        if work > MAX_NODES * 4 || pending.len() > MAX_NODES * 4 {
            return Ok(true);
        }
        if facts.local(part) == Some(symbol) {
            return Ok(true);
        }
        let subplace = matches!(part.kind, NodeKind::IndexAccess | NodeKind::FieldAccess)
            || (part.kind == NodeKind::Operator && part.operator.as_deref() == Some("deref"));
        let value = matches!(part.kind, NodeKind::PatternLiteral | NodeKind::PatternRange);
        if !subplace && !value {
            pending.extend(facts.body.children(part)?);
        }
    }
    Ok(false)
}

fn root<'a>(facts: &Facts<'_, 'a>, node: &Node, depth: u8) -> Option<&'a str> {
    let depth = depth.checked_sub(1)?;
    match node.kind {
        NodeKind::Identifier => facts.local(node),
        NodeKind::IndexAccess | NodeKind::FieldAccess | NodeKind::Reference => {
            root(facts, facts.body.child(node, 0).ok()?, depth)
        }
        NodeKind::Operator if node.operator.as_deref() == Some("deref") => {
            root(facts, facts.body.child(node, 0).ok()?, depth)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{fresh_local, writes_binding};
    use crate::advisory::runtime_bounds::MAX_NODES;
    use crate::cpg::bounds::facts::Facts;
    use crate::cpg::model::{Document, NodeKind};
    use crate::cpg::source::Sources;
    use crate::cpg::tree::{Body, Tables};

    fn document() -> Document {
        serde_json::from_str(include_str!(
            "../../../tests/testdata/native-output-quote-args-go.cpg.json"
        ))
        .unwrap()
    }

    fn with_facts(document: &Document, check: impl FnOnce(&Facts<'_, '_>)) {
        let unit = document.unit.as_ref().unwrap();
        let function = &unit.functions.as_ref().unwrap()[0];
        let body = Body::new(function).unwrap();
        let tables = Tables::new(unit).unwrap();
        let sources = Sources::new(unit).unwrap();
        let facts = Facts {
            body: &body,
            tables: &tables,
            sources: &sources,
            pointer_bits: 64,
            destructors_are_calls: false,
        };
        check(&facts);
    }

    #[test]
    fn detached_cycles_exhaust_write_inspection_without_certifying_storage() {
        let mut document = document();
        let unit = document.unit.as_mut().unwrap();
        let function = &mut unit.functions.as_mut().unwrap()[0];
        let mut cycle = function.nodes[0].clone();
        cycle.id = "detached.cycle".into();
        cycle.kind = NodeKind::Tuple;
        cycle.children = vec![cycle.id.clone()];
        function.nodes.push(cycle);

        // Parent uniqueness alone permits a disconnected component with a cycle.
        with_facts(&document, |facts| {
            let target = facts.body.node("detached.cycle").unwrap();
            assert_eq!(writes_binding(facts, target, "unrelated.symbol"), Ok(true));
        });
    }

    #[test]
    fn oversized_place_rosters_reject_storage_before_scanning_metadata() {
        let mut document = document();
        let function = &mut document.unit.as_mut().unwrap().functions.as_mut().unwrap()[0];
        let place = function.places[0].clone();
        function.places = (0..=MAX_NODES * 4)
            .map(|index| {
                let mut entry = place.clone();
                entry.id = format!("oversized.{index}");
                entry
            })
            .collect();

        with_facts(&document, |facts| {
            let base = facts
                .body
                .function
                .nodes
                .iter()
                .find(|node| facts.local(node) == Some("f4.s1"))
                .unwrap();
            assert!(!fresh_local(facts, base));
        });
    }
}
