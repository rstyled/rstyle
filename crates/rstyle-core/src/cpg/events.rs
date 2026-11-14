//! Calls, call edges, effects and mutations of one function, as the profile's Rule
//! facts define them.

use super::effects::EffectClasses;
use super::model::{
    Definition, DefinitionKind, Dispatch, Node, NodeKind, Origin, Place, PlaceMode, Ref, RefStatus,
    RefTarget, Scope, Step, Symbol, TargetKind, TypeKind,
};
use super::source::Sources;
use super::tree::{Body, Tables, anchor, generated, is_kind, line, scope_end};
use crate::complexity::{Kind, Node as Construct};
use crate::facts::{
    ArgSource, CallEvent, Edge, EffectEvent, EffectKind, LibraryClass, MutationEvent,
    MutationOrigin,
};
use crate::paths::strip_generics;
use crate::wording::Message;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Events {
    pub calls: Vec<CallEvent>,
    pub edges: Vec<Edge>,
    pub effects: Vec<EffectEvent>,
    pub recursions: Vec<Construct>,
}

/// Whether a call has no callee node, so that its receiver is its first argument.
fn method_form(reference: &Ref) -> bool {
    matches!(
        reference.dispatch,
        Some(Dispatch::Method | Dispatch::Dynamic)
    )
}

/// Read-only context of one function's derivations.
pub struct Walk<'a> {
    pub body: &'a Body<'a>,
    pub tables: &'a Tables<'a>,
    pub sources: &'a Sources<'a>,
    owner: &'a str,
    params: BTreeMap<&'a str, usize>,
    classes: &'a EffectClasses,
}

/// A node being walked, and whether a user-written `unsafe` block encloses it.
#[derive(Clone, Copy)]
struct Site<'a> {
    node: &'a Node,
    in_unsafe: bool,
}

/// A call's declared callee, the implementation it statically selects, and its reference.
struct Callee<'a> {
    declared: &'a Definition,
    selected: &'a Definition,
    dynamic: bool,
    reference: &'a Ref,
}

enum Call<'a> {
    Skipped,
    Indirect(&'static str),
    Resolved(Callee<'a>),
}

fn user_unsafe(node: &Node) -> bool {
    node.kind == NodeKind::Block && !node.synthetic && node.modifiers.iter().any(|m| m == "unsafe")
}

impl<'a> Walk<'a> {
    pub fn new(
        body: &'a Body<'a>,
        tables: &'a Tables<'a>,
        sources: &'a Sources<'a>,
        classes: &'a EffectClasses,
    ) -> Self {
        let signature = &body.function.signature;
        let params = signature
            .parameters
            .iter()
            .map(|p| (p.symbol.as_str(), p.position))
            .collect();
        Self {
            body,
            tables,
            sources,
            owner: &body.function.definition,
            params,
            classes,
        }
    }

    /// Calls, edges, effects and recursions in tree pre-order.
    pub fn events(&self) -> Result<Events, String> {
        let mut out = Events::default();
        let root = Site {
            node: self.body.root()?,
            in_unsafe: false,
        };
        self.walk(root, &mut out)?;
        Ok(out)
    }

    fn line(&self, node: &Node) -> Result<usize, String> {
        line(self.sources, node)
    }

    fn effect(&self, kind: EffectKind, detail: String, node: &Node) -> Result<EffectEvent, String> {
        Ok(EffectEvent {
            kind,
            detail,
            line: self.line(node)?,
        })
    }

    /// A node's own events, then its children's.
    fn walk(&self, site: Site<'a>, out: &mut Events) -> Result<(), String> {
        self.own_events(site, out)?;
        let in_unsafe = site.in_unsafe || user_unsafe(site.node);

        for node in self.body.children(site.node)? {
            self.walk(Site { node, in_unsafe }, out)?;
        }
        Ok(())
    }

    fn own_events(&self, site: Site<'a>, out: &mut Events) -> Result<(), String> {
        let node = site.node;
        match node.kind {
            NodeKind::Call => self.call(site, out),
            NodeKind::Identifier => self.path(node, out),
            NodeKind::Block if user_unsafe(node) => {
                out.effects
                    .push(self.effect(EffectKind::Unsafe, "unsafe block".into(), node)?);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A reference to a static that can be written is a global effect, read or written.
    fn path(&self, node: &Node, out: &mut Events) -> Result<(), String> {
        let target = self
            .body
            .reference(&node.id)
            .and_then(|r| r.target.as_ref());
        let Some(target) = target.filter(|t| t.kind == TargetKind::Definition) else {
            return Ok(());
        };

        let definition = self.tables.definition(target_id(target)?)?;
        if definition.kind == DefinitionKind::Static && has_modifier(definition, "mutable") {
            let name = strip_generics(&definition.name);
            let detail = self
                .tables
                .wording()
                .say(Message::WritableStatic, &[("name", &name)]);
            out.effects
                .push(self.effect(EffectKind::Global, detail, node)?);
        }
        Ok(())
    }

    fn call(&self, site: Site<'a>, out: &mut Events) -> Result<(), String> {
        match self.classify_call(site.node)? {
            Call::Skipped => Ok(()),
            Call::Indirect(what) => {
                let line = self.line(site.node)?;
                out.calls.push(CallEvent {
                    callee: what.into(),
                    line,
                    dynamic: true,
                    library: None,
                    library_callee: None,
                });
                Ok(())
            }
            Call::Resolved(callee) => self.resolved(site, callee, out),
        }
    }

    fn classify_call(&self, node: &'a Node) -> Result<Call<'a>, String> {
        // Implicit calls (destructors, conversions) have no call site in the source.
        if node.provenance.origin == Origin::Analysis || scope_end(node) {
            return Ok(Call::Skipped);
        }
        let Some(reference) = self.body.reference(&node.id) else {
            return Ok(Call::Indirect("<unresolved>"));
        };
        match reference.status {
            RefStatus::Unresolved => Ok(Call::Indirect("<unresolved>")),
            RefStatus::Dependent => {
                let missing = || format!("Ref at {} has no via", node.id);
                let declared = self
                    .tables
                    .definition(reference.via.as_deref().ok_or_else(missing)?)?;
                let selected = declared;
                Ok(Call::Resolved(Callee {
                    declared,
                    selected,
                    dynamic: false,
                    reference,
                }))
            }
            RefStatus::Resolved => self.resolved_target(node, reference),
        }
    }

    fn resolved_target(&self, node: &'a Node, reference: &'a Ref) -> Result<Call<'a>, String> {
        let missing = || format!("Ref at {} has no target", node.id);
        let target = reference.target.as_ref().ok_or_else(missing)?;
        if reference.dispatch == Some(Dispatch::FunctionValue) || target.kind == TargetKind::Symbol
        {
            return self.function_value(node, reference);
        }
        if !matches!(
            target.kind,
            TargetKind::Definition | TargetKind::LibraryOperation
        ) {
            return Ok(Call::Skipped);
        }

        let declared = self.tables.definition(target_id(target)?)?;
        let selected = match &reference.selected {
            Some(selected) => self.tables.definition(target_id(selected)?)?,
            None => declared,
        };
        let dynamic = reference.dispatch == Some(Dispatch::Dynamic);
        Ok(Call::Resolved(Callee {
            declared,
            selected,
            dynamic,
            reference,
        }))
    }

    /// Calls of a closure defined here are part of this body; other callables are unknown.
    fn function_value(&self, node: &Node, reference: &Ref) -> Result<Call<'a>, String> {
        if reference.closure.is_some() {
            return Ok(Call::Skipped);
        }

        let callee = self.body.source_children(node)?.into_iter().next();
        let ty = callee.and_then(|callee| callee.ty.as_deref());
        let pointer = match ty {
            Some(ty) => self.tables.ty(ty)?.kind == TypeKind::Function,
            None => false,
        };
        Ok(Call::Indirect(if pointer {
            "<fn pointer>"
        } else {
            "<indirect callable>"
        }))
    }

    fn resolved(&self, site: Site<'a>, callee: Callee<'a>, out: &mut Events) -> Result<(), String> {
        if !matches!(
            callee.selected.kind,
            DefinitionKind::Function | DefinitionKind::Method
        ) {
            return Ok(());
        }
        self.edge(site.node, &callee, out)?;
        let (declared, dynamic) = (callee.declared, callee.dynamic);
        if declared.scope == Scope::Unit {
            return Ok(());
        }

        let name = strip_generics(&declared.name);
        if !site.in_unsafe && !generated(site.node) && has_modifier(declared, "unsafe") {
            let detail = format!("call to unsafe fn {name}");
            out.effects
                .push(self.effect(EffectKind::Unsafe, detail, site.node)?);
        }
        let line = self.line(site.node)?;
        let library = self.library_class(site.node, &callee)?;
        let library_callee =
            (callee.selected.scope == Scope::Library).then(|| declared.stable_identity.clone());
        out.calls.push(CallEvent {
            callee: name,
            line,
            dynamic,
            library,
            library_callee,
        });
        Ok(())
    }

    /// What the namespace's contract says this library call does: the implementation it selects,
    /// when the document names a library's, since that's what runs, and its declared callee
    /// otherwise. An implementation in the workspace is followed through the call edge instead. An
    /// entry that only holds for a call that runs none of the workspace's code holds where the
    /// call's reference says it calls nothing back.
    fn library_class(
        &self,
        node: &'a Node,
        callee: &Callee<'a>,
    ) -> Result<Option<LibraryClass>, String> {
        if callee.selected.scope != Scope::Library {
            return Ok(None);
        }
        let runs = callee.selected;

        let Some(operation) = runs.operation.as_deref() else {
            return Ok(self.classes.unlisted());
        };
        if self.writes_to_memory(node, operation)? {
            return Ok(Some(LibraryClass::Pure));
        }
        let calls_back = callee.reference.calls_back.as_ref();
        Ok(if calls_back.is_some_and(|calls_back| !calls_back.value) {
            self.classes.class_without_callback(operation)
        } else {
            self.classes.class(operation)
        })
    }

    /// Whether the contract keeps this call's effect in a buffer, by an argument's type. The
    /// argument is counted after the callee or a method call's receiver, by the parameter it binds to.
    fn writes_to_memory(&self, node: &'a Node, operation: &str) -> Result<bool, String> {
        let Some(rule) = self.classes.memory_write(operation) else {
            return Ok(false);
        };

        let argument = match self.body.reference(&node.id) {
            Some(reference) if reference.arguments.is_some() => {
                let position = rule.argument + usize::from(method_form(reference));
                let arguments = self.body.arguments(node)?.into_iter();
                let mut bound = arguments.filter(|argument| argument.position == Some(position));
                bound.next().map(|argument| argument.value)
            }
            _ => self
                .body
                .source_children(node)?
                .get(1 + rule.argument)
                .copied(),
        };
        let Some(ty) = argument.and_then(|argument| argument.ty.as_deref()) else {
            return Ok(false);
        };

        let spelling = &self.tables.ty(ty)?.spelling;
        Ok(rule.types.iter().any(|name| name == spelling))
    }

    /// Workspace calls, same unit included, form the call graph.
    fn edge(&self, node: &'a Node, callee: &Callee<'a>, out: &mut Events) -> Result<(), String> {
        if callee.selected.id == self.owner && !generated(node) {
            let (line, column) = self.sources.location(own_span(node)?)?;
            let recursion = Construct {
                kind: Kind::Recursion,
                line,
                column,
                children: vec![],
            };
            out.recursions.push(recursion);
        }
        if callee.selected.scope == Scope::Library {
            return Ok(());
        }

        let args = self.arguments(node, callee.reference, callee.declared)?;
        let line = self.line(node)?;
        let callee_id = callee.selected.stable_identity.clone();
        out.edges.push(Edge {
            callee: callee_id,
            line,
            dynamic: callee.dynamic,
            required: has_modifier(callee.selected, "abstract"),
            args,
        });
        Ok(())
    }

    /// Arguments after the callee, or after the receiver of a method call; a method
    /// called by path takes its receiver as the first argument, which is not counted.
    /// Where the call binds them, each is placed at the parameter it binds to, except in a
    /// method called by path: Python's class call is one too, and passes no receiver, so
    /// which argument is the receiver isn't known, and they stay in written order.
    fn arguments(
        &self,
        node: &'a Node,
        reference: &Ref,
        declared: &Definition,
    ) -> Result<Vec<ArgSource>, String> {
        let method_form = method_form(reference);
        let receiver = usize::from(!method_form && declared.kind == DefinitionKind::Method);
        if reference.arguments.is_some() && receiver == 0 {
            return self.bound_arguments(node, usize::from(method_form));
        }

        let children = self.body.source_children(node)?;
        children
            .iter()
            .skip(1 + receiver)
            .map(|arg| self.classify(arg))
            .collect()
    }

    /// Arguments placed by the parameter each binds to, from `first` on: one with no position
    /// is left out, and a parameter no argument binds, which takes its default, is other.
    fn bound_arguments(&self, node: &'a Node, first: usize) -> Result<Vec<ArgSource>, String> {
        let mut placed: BTreeMap<usize, ArgSource> = BTreeMap::new();
        for argument in self.body.arguments(node)? {
            let Some(at) = argument.position.and_then(|it| it.checked_sub(first)) else {
                continue;
            };
            placed.entry(at).or_insert(self.classify(argument.value)?);
        }

        let count = placed.keys().next_back().map_or(0, |last| last + 1);
        let at = |position| placed.get(&position).copied().unwrap_or(ArgSource::Other);
        Ok((0..count).map(at).collect())
    }

    fn classify(&self, arg: &'a Node) -> Result<ArgSource, String> {
        let arg = self.peel(arg)?;
        let target = self.body.reference(&arg.id).and_then(|r| r.target.as_ref());

        if arg.kind == NodeKind::Identifier
            && let Some(target) = target.filter(|t| t.kind == TargetKind::Symbol)
            && let Some(position) = self.params.get(target_id(target)?)
        {
            return Ok(ArgSource::Param(*position));
        }
        if arg.kind == NodeKind::Literal || self.is_constant(arg, target)? {
            return Ok(ArgSource::Const);
        }
        if arg.kind == NodeKind::FieldAccess && self.local_field_chain(arg)? {
            return Ok(ArgSource::Field);
        }
        Ok(ArgSource::Other)
    }

    fn is_constant(&self, arg: &Node, target: Option<&RefTarget>) -> Result<bool, String> {
        let Some(target) = target.filter(|_| arg.kind == NodeKind::Identifier) else {
            return Ok(false);
        };
        Ok(match target.kind {
            TargetKind::Constructor | TargetKind::Variant => true,
            TargetKind::Definition => matches!(
                self.tables.definition(target_id(target)?)?.kind,
                DefinitionKind::Constant | DefinitionKind::Static
            ),
            _ => false,
        })
    }

    /// `a.b.c` whose root is a local binding.
    fn local_field_chain(&self, node: &'a Node) -> Result<bool, String> {
        let node = self.peel(node)?;
        if node.kind != NodeKind::FieldAccess {
            return Ok(false);
        }
        let Some(base) = self.body.source_children(node)?.into_iter().next() else {
            return Ok(false);
        };

        let base = self.peel(base)?;
        let local = self
            .body
            .reference(&base.id)
            .and_then(|r| r.target.as_ref())
            .map(|t| t.kind);
        Ok(local == Some(TargetKind::Symbol) || self.local_field_chain(base)?)
    }

    /// Borrows and dereferences change how a value is passed, not what it is.
    fn peel(&self, mut node: &'a Node) -> Result<&'a Node, String> {
        while is_kind(node, &[NodeKind::Reference]) || node.operator.as_deref() == Some("deref") {
            match self.body.source_children(node)?.as_slice() {
                [inner] => node = inner,
                _ => break,
            }
        }
        Ok(node)
    }
}

fn target_id(target: &RefTarget) -> Result<&str, String> {
    target
        .id
        .as_deref()
        .ok_or_else(|| format!("A {:?} target has no ID", target.kind))
}

fn has_modifier(definition: &Definition, modifier: &str) -> bool {
    definition.modifiers.iter().any(|m| m == modifier)
}

fn own_span(node: &Node) -> Result<&super::model::FileSpan, String> {
    node.span
        .as_ref()
        .ok_or_else(|| format!("Node {} has no span", node.id))
}

/// Writes and exclusive borrows of places rooted at a binding, in place order.
/// Mutation events in place order, each with the symbol its place is rooted at.
pub fn mutations<'a>(walk: &Walk<'a>) -> Result<Vec<(MutationEvent, &'a Symbol)>, String> {
    let mut events = Vec::new();
    for place in &walk.body.function.places {
        if let Some(event) = mutation(walk, place)? {
            events.push(event);
        }
    }
    Ok(events)
}

fn mutation<'a>(
    walk: &Walk<'a>,
    place: &Place,
) -> Result<Option<(MutationEvent, &'a Symbol)>, String> {
    let write = place.access != super::model::AccessMode::Read;
    let borrow = place.mode == PlaceMode::Exclusive;
    let Some(root) = place.root.symbol.as_deref().filter(|_| write || borrow) else {
        return Ok(None);
    };
    let symbol = walk.body.symbol(root)?;

    let unknown = || {
        format!(
            "Place {} is mutated but does not say whether the write escapes",
            place.id
        )
    };
    let escapes = place.escapes.ok_or_else(unknown)?;
    if !escapes && declared_by_macro(walk, symbol)? {
        return Ok(None);
    }
    let delegated = escapes && borrow && !write && reborrows_workspace_object(walk, place)?;
    let fields: Vec<&str> = place.path.iter().filter_map(field_name).collect();
    let line = walk
        .sources
        .location(anchor(walk.body.node(&place.node)?)?)?
        .0;
    let event = MutationEvent {
        origin: binding_origin(walk, symbol.node.as_deref())?,
        root: symbol.name.clone(),
        field_path: fields.join("."),
        line,
        escapes: escapes && !delegated,
        delegated,
    };
    Ok(Some((event, symbol)))
}

fn field_name(step: &Step) -> Option<&str> {
    match step {
        Step::Field(field) => Some(&field.field),
        _ => None,
    }
}

/// `&mut *r` of a whole workspace record or sum: the callee performs the writes.
fn reborrows_workspace_object(walk: &Walk<'_>, place: &Place) -> Result<bool, String> {
    if !matches!(place.path.last(), Some(Step::Deref(_))) {
        return Ok(false);
    }
    let ty = place
        .ty
        .as_deref()
        .ok_or_else(|| format!("Place {} reborrows a value of unknown type", place.id))?;
    let ty = walk.tables.ty(ty)?;

    let Some(definition) = ty
        .definition
        .as_deref()
        .filter(|_| matches!(ty.kind, TypeKind::Record | TypeKind::Sum))
    else {
        return Ok(false);
    };
    Ok(walk.tables.definition(definition)?.scope != Scope::Library)
}

/// A binding a library macro declares is that macro's working state, not the developer's.
/// A body binding is declared by its node; a parameter or receiver, by its own record.
fn declared_by_macro(walk: &Walk<'_>, symbol: &Symbol) -> Result<bool, String> {
    let provenance = match symbol.node.as_deref() {
        Some(node) => &walk.body.node(node)?.provenance,
        None => {
            let signature = &walk.body.function.signature;
            let declared = signature
                .parameters
                .iter()
                .filter(|parameter| {
                    parameter.symbol == symbol.id
                        || parameter
                            .bindings
                            .iter()
                            .flatten()
                            .any(|binding| *binding == symbol.id)
                })
                .map(|parameter| parameter.provenance.as_ref())
                .chain(
                    signature
                        .receiver
                        .iter()
                        .filter(|receiver| receiver.symbol == symbol.id)
                        .map(|receiver| receiver.provenance.as_ref()),
                )
                .next()
                .flatten();
            let Some(provenance) = declared else {
                return Ok(false);
            };
            provenance
        }
    };
    match (provenance.origin, provenance.macro_definition.as_deref()) {
        (Origin::MacroExpansion, Some(definition)) => {
            Ok(walk.tables.definition(definition)?.scope == Scope::Library)
        }
        _ => Ok(false),
    }
}

/// Bindings a `for` loop's lowering introduces are excluded from source budgets.
fn binding_origin(walk: &Walk<'_>, node: Option<&str>) -> Result<MutationOrigin, String> {
    let Some(node) = node else {
        return Ok(MutationOrigin::Source);
    };
    let provenance = &walk.body.node(node)?.provenance;
    let lowered =
        provenance.origin == Origin::Desugaring && provenance.construct.as_deref() == Some("for");
    Ok(if lowered {
        MutationOrigin::ForDesugaring
    } else {
        MutationOrigin::Source
    })
}
