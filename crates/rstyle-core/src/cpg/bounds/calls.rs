//! Call-site facts for summary composition: formal provenance, targets and argument
//! sizes, and implicit calls that run without a call site.

use super::super::model::{Definition, Node, RefStatus, Type, TypeKind};
use super::Collector;
use super::extent::Place;
use super::facts::is_shared_reference;
use crate::runtime_bounds::{Argument, Callee, Extent, Formal, Integer, Measure, Root, Value};

/// A call's target and, for a standard-library body, its operation key.
pub(super) type Resolution = (Callee, Option<String>);

/// A parameter in declaration order: its symbol when it binds a single name.
pub(super) struct Parameter<'a> {
    pub symbol: Option<&'a str>,
    pub name: &'a str,
}

impl<'a> Collector<'_, '_, 'a> {
    /// The receiver, then each parameter.
    pub(super) fn parameter_symbols(&self) -> Result<Vec<Parameter<'a>>, String> {
        let signature = &self.facts.body.function.signature;
        let mut parameters = Vec::new();
        if let Some(receiver) = &signature.receiver {
            let symbol = self.facts.body.symbol(&receiver.symbol)?;
            parameters.push(Parameter {
                symbol: Some(receiver.symbol.as_str()),
                name: symbol.name.as_str(),
            });
        }
        for parameter in &signature.parameters {
            let symbol = self.facts.body.symbol(&parameter.symbol)?;
            let single = symbol.declared.is_some();
            parameters.push(Parameter {
                symbol: single.then_some(parameter.symbol.as_str()),
                name: if single { symbol.name.as_str() } else { "_" },
            });
        }

        Ok(parameters)
    }

    pub(super) fn formal_positions(&mut self, parameters: &[Parameter<'a>]) {
        for (position, parameter) in parameters.iter().enumerate() {
            if let Some(symbol) = parameter.symbol {
                self.formals.insert(symbol, position as u32);
            }
        }
    }

    pub(super) fn parameters(&mut self, parameters: &[Parameter<'a>]) -> Result<(), String> {
        for parameter in parameters {
            let formal = self.parameter(parameter)?;
            self.program.formals.push(formal);
        }
        Ok(())
    }

    fn parameter(&mut self, parameter: &Parameter<'a>) -> Result<Formal, String> {
        let Some(symbol) = parameter.symbol else {
            return Ok(Formal {
                name: "_".into(),
                measure: Measure::Opaque,
            });
        };

        let binding = self.facts.body.symbol(symbol)?;
        let ty = binding.ty.as_deref().and_then(|ty| self.facts.type_of(ty));
        let label = parameter.name.to_string();
        let measure = match self.facts.integer(ty) {
            Some(integer) => {
                let variable = self.variable(label.clone(), Value::Input(label.clone()), integer);
                self.program.inputs.push(label.clone());
                self.locals.insert(symbol, variable);
                Measure::Scalar {
                    input: label.clone(),
                }
            }
            None if !binding.mutable && is_shared_reference(ty) => Measure::Shared,
            None => Measure::Opaque,
        };
        Ok(Formal {
            name: label,
            measure,
        })
    }

    pub(super) fn root(&self, symbol: &str) -> Root {
        self.formals
            .get(symbol)
            .map_or(Root::Local(self.number(symbol)), |&position| {
                Root::Formal(position)
            })
    }

    pub(super) fn provenance(&self, place: &Place<'a>) -> Extent {
        Extent {
            input: place.label.clone(),
            root: self.root(place.root),
            fields: place.path.clone(),
        }
    }

    // Formal positions first: extent provenance records formal-rooted places.
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    pub(super) fn install_extents(
        &mut self,
        parameters: &[Parameter<'a>],
        places: Vec<Place<'a>>,
    ) -> Result<(), String> {
        self.formal_positions(parameters);
        for place in places {
            let length = Integer::unsigned(self.facts.pointer_bits);
            self.variable(
                place.label.clone(),
                Value::Input(place.label.clone()),
                length,
            );
            self.program.inputs.push(place.label.clone());
            self.program.extents.push(self.provenance(&place));
            self.extents.insert(place.key(), place.label);
        }
        Ok(())
    }

    /// The target of a call, method call or overloaded operator node, by its reference.
    pub(super) fn reference_target(&self, node: &Node) -> Resolution {
        let Some(reference) = self.facts.reference(node) else {
            return (Callee::Unresolved, None);
        };
        let Some(declared) = self.facts.declared(reference) else {
            return (Callee::Unresolved, None);
        };
        if !self.facts.interface_member(declared) {
            let operation = declared.operation.clone();
            return self.classified(
                declared,
                operation,
                reference.calls_back.as_ref().map(|c| c.value),
            );
        }
        // Interface members resolve only through static selection of an implementation.
        if reference.status == RefStatus::Dependent {
            return (Callee::Unresolved, None);
        }
        let Some(selected) = reference.selected.as_ref() else {
            return (Callee::Unresolved, None);
        };

        let Some(implementation) = selected
            .id
            .as_deref()
            .and_then(|id| self.facts.tables.definition(id).ok())
        else {
            return (Callee::Unresolved, None);
        };
        let calls_back = reference.calls_back.as_ref().map(|c| c.value);
        // An interface's own default body has no implementation key.
        let operation = (!self.facts.interface_member(implementation))
            .then(|| implementation.operation.clone())
            .flatten();
        self.classified(implementation, operation, calls_back)
    }

    // Standard bodies are excluded only when they cannot call back into workspace code;
    // those keep the implementation's operation key.
    pub(super) fn classified(
        &self,
        callee: &Definition,
        operation: Option<String>,
        calls_back: Option<bool>,
    ) -> Resolution {
        if !callee.standard {
            let compilation = callee.compilation.clone().unwrap_or_default();
            return (
                Callee::Resolved(format!("{compilation}/{}", callee.stable_identity)),
                None,
            );
        }
        if calls_back.unwrap_or(true) {
            (Callee::Unresolved, None)
        } else {
            (Callee::Library(callee.stable_identity.clone()), operation)
        }
    }

    /// Workspace code that runs without a call site: a workspace type's own `Deref`
    /// reached by an implicit dereference, and drop glue.
    pub(super) fn hidden_call(&self, expression: &Node) -> Result<Option<&'static str>, String> {
        if self.workspace_deref(expression) {
            return Ok(Some("Overloaded auto-deref runs a workspace Deref impl"));
        }
        Ok(self
            .facts
            .ty(expression)
            .is_some_and(|ty| self.drop_runs_workspace_code(ty, 64))
            .then_some("A value with a workspace Drop impl runs uncounted drop glue"))
    }

    /// Whether dropping a value of `ty` may run a workspace `Drop` impl. A reference or a
    /// pointer that doesn't own its referent drops nothing behind it.
    fn drop_runs_workspace_code(&self, ty: &Type, depth: usize) -> bool {
        let Some(depth) = depth.checked_sub(1) else {
            return false;
        };
        let borrowed = ty.kind == TypeKind::Reference
            || (ty.kind == TypeKind::Pointer && ty.mode.as_deref() != Some("owned"));
        if borrowed {
            return false;
        }

        self.workspace_destructor(ty)
            || components(ty)
                .filter_map(|id| self.facts.type_of(id))
                .any(|part| self.drop_runs_workspace_code(part, depth))
    }

    fn workspace_deref(&self, expression: &Node) -> bool {
        let Some(adjustments) = &expression.adjustments else {
            return false;
        };
        let mut current = self.facts.ty(expression);
        for adjustment in adjustments {
            if adjustment.kind == "overloaded_deref"
                && current.is_some_and(|ty| self.workspace_adt(ty))
            {
                return true;
            }
            current = adjustment
                .target
                .as_deref()
                .and_then(|id| self.facts.type_of(id));
        }
        false
    }

    // Only the dereferenced type's own impl runs; `Vec<Workspace>` derefs through std.
    fn workspace_adt(&self, mut ty: &'a Type) -> bool {
        while ty.kind == TypeKind::Reference {
            match ty.target.as_deref().and_then(|id| self.facts.type_of(id)) {
                Some(target) => ty = target,
                None => return false,
            }
        }
        self.nominal(ty)
            .is_some_and(|definition| !definition.standard)
    }

    fn nominal(&self, ty: &Type) -> Option<&Definition> {
        let nominal = matches!(ty.kind, TypeKind::Record | TypeKind::Sum)
            || (ty.kind == TypeKind::Pointer && ty.mode.as_deref() == Some("owned"));
        let id = ty.definition.as_deref().filter(|_| nominal)?;
        self.facts.tables.definition(id).ok()
    }

    /// A nominal type the workspace defines with a destructor of its own.
    fn workspace_destructor(&self, ty: &Type) -> bool {
        self.nominal(ty)
            .is_some_and(|definition| !definition.standard && definition.destructor)
    }

    /// Scalars keep their captured value; shared places keep their root and field path.
    pub(super) fn argument(&self, argument: &'a Node) -> Result<Argument, String> {
        if self.facts.integer(self.facts.adjusted(argument)).is_some() {
            return Ok(Argument::Scalar(self.value(argument)));
        }
        Ok(match self.place(argument)? {
            Some(place) => Argument::Place {
                root: self.root(place.root),
                fields: place.path.clone(),
            },
            None => Argument::Opaque,
        })
    }
}

/// The types a type is built from. A function item's type is built from its generic
/// arguments, not its signature; a function pointer's and a closure's from their
/// signatures.
pub(super) fn components(ty: &Type) -> impl Iterator<Item = &str> {
    let item = ty.kind == TypeKind::Function && ty.definition.is_some();
    let signature = if item { &None } else { &ty.parameters };
    let result = if item { &None } else { &ty.result };
    let lists = [&ty.arguments, &ty.items, signature];
    let singles = [&ty.element, &ty.target, result];

    lists
        .into_iter()
        .flatten()
        .flatten()
        .chain(singles.into_iter().flatten())
        .map(String::as_str)
}
