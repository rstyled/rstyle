//! Durable source values derived from the completed declaration index.
use super::{Candidate, MethodOrigin, capabilities::Wrapper, index::CheckedDeclaration, shapes};
use crate::{ByteSpan, source_evidence};
use std::{collections::BTreeMap, str::FromStr};
use syn::visit::Visit;

pub struct Declaration {
    pub qualified_name: String,
    pub kind: &'static str,
    pub source: String,
}

pub struct Selection {
    pub owner: Declaration,
    pub method: Declaration,
    pub origin: Option<(Declaration, String)>,
}

pub struct Dispatch {
    pub caller: Declaration,
    pub enumeration: Declaration,
    pub arms: Vec<(String, Selection)>,
}

pub struct Capability {
    pub caller: Declaration,
    pub selections: Vec<Selection>,
}

pub(super) fn dispatch(
    declaration: CheckedDeclaration<'_>,
    candidate: &Candidate,
) -> Result<Dispatch, String> {
    check_caller(&declaration, candidate.caller_header)?;
    let enumeration = nominal(&declaration, &candidate.enum_name, "enum")?;
    let arms = candidate
        .arms
        .iter()
        .map(|arm| {
            let selected = selection(&declaration, arm.callee_header, &arm.origin)?;
            if !selected
                .owner
                .qualified_name
                .ends_with(&format!("::{}", arm.receiver_type))
                || !selected
                    .method
                    .qualified_name
                    .ends_with(&format!("::{}", arm.method))
            {
                return Err("Review dispatch selection differs from its completed witness".into());
            }
            Ok((arm.variant.clone(), selected))
        })
        .collect::<Result<_, String>>()?;
    Ok(Dispatch {
        caller: caller(&declaration)?,
        enumeration,
        arms,
    })
}

pub(super) fn capability(
    declaration: CheckedDeclaration<'_>,
    wrapper: &Wrapper,
) -> Result<Capability, String> {
    check_caller(&declaration, wrapper.caller_header)?;
    if wrapper.owner.source_owner != declaration.owner_index
        || wrapper.owner.module != declaration.module_index
        || wrapper.owner.name != wrapper.owner_name
    {
        return Err("Review wrapper belongs to a different original source owner".into());
    }
    let selections = wrapper
        .methods
        .iter()
        .map(|method| selection(&declaration, method.callee_header, &method.origin))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Capability {
        caller: caller(&declaration)?,
        selections,
    })
}

fn check_caller(declaration: &CheckedDeclaration<'_>, header: ByteSpan) -> Result<(), String> {
    if shapes::function_header(declaration.source, declaration.function) != header {
        return Err("Review caller differs from its completed declaration witness".into());
    }
    Ok(())
}

fn caller(declaration: &CheckedDeclaration<'_>) -> Result<Declaration, String> {
    let (qualified_name, kind) = location(declaration);
    Ok(Declaration {
        qualified_name,
        kind,
        source: normalized_caller(declaration)?,
    })
}

/// Checked declaration identity even when the detector has no positive proposal.
pub(super) fn location(declaration: &CheckedDeclaration<'_>) -> (String, &'static str) {
    let header = shapes::function_header(declaration.source, declaration.function);
    let method = declaration.module.review_method(header).ok();
    let (name, kind) = match method {
        Some((owner, name, origin)) => (method_name(owner, name, origin), "method"),
        None => (declaration.function.sig.ident.to_string(), "free-function"),
    };

    let namespace = &declaration.module.namespace;
    let qualified = if namespace.is_empty() {
        format!("crate::{name}")
    } else {
        format!("crate::{namespace}::{name}")
    };
    (qualified, kind)
}

fn selection(
    declaration: &CheckedDeclaration<'_>,
    header: ByteSpan,
    origin: &MethodOrigin,
) -> Result<Selection, String> {
    let (owner, name, selected_origin) = declaration.module.review_method(header)?;
    let owner = nominal(declaration, owner, "struct")?;
    let origin = match (origin, selected_origin) {
        (MethodOrigin::Inherent, None) => None,
        (
            MethodOrigin::LocalTrait {
                trait_name, member, ..
            },
            Some(selected),
        ) if trait_name == selected && member == name => {
            Some((nominal(declaration, selected, "trait")?, member.clone()))
        }
        _ => return Err("Review method origin differs from its completed witness".into()),
    };

    let method = record(
        declaration,
        &method_name(
            declaration.module.review_method(header)?.0,
            name,
            selected_origin,
        ),
        "method",
        header,
    )?;
    Ok(Selection {
        owner,
        method,
        origin,
    })
}

fn method_name(owner: &str, name: &str, origin: Option<&str>) -> String {
    match origin {
        Some(origin) => format!("<{owner} as {origin}>::{name}"),
        None => format!("{owner}::{name}"),
    }
}

fn nominal(
    declaration: &CheckedDeclaration<'_>,
    name: &str,
    kind: &'static str,
) -> Result<Declaration, String> {
    let matches = declaration
        .module
        .review
        .iter()
        .filter(|item| item.name == name && item.kind == kind)
        .collect::<Vec<_>>();
    let [item] = matches.as_slice() else {
        return Err("Review nominal declaration is ambiguous or absent".into());
    };
    record(declaration, name, kind, item.range)
}

fn record(
    declaration: &CheckedDeclaration<'_>,
    name: &str,
    kind: &'static str,
    range: ByteSpan,
) -> Result<Declaration, String> {
    let qualified_name = format!(
        "crate::{}{}",
        if declaration.module.namespace.is_empty() {
            String::new()
        } else {
            format!("{}::", declaration.module.namespace)
        },
        name
    );
    let source = source_evidence::byte_range(declaration.source, range)?;
    Ok(Declaration {
        qualified_name,
        kind,
        source: canonical(source)?,
    })
}

/// Normalize only original binding declarations and their scoped value paths.
/// Names in type paths, variants, field selectors and selected method names stay exact.
fn normalized_caller(declaration: &CheckedDeclaration<'_>) -> Result<String, String> {
    let function = declaration.function;
    let mut bindings = Bindings {
        source: declaration.source,
        names: BTreeMap::new(),
        replacements: BTreeMap::new(),
        next: 0,
    };
    for (position, argument) in function.sig.inputs.iter().enumerate() {
        if let syn::FnArg::Typed(argument) = argument
            && let syn::Pat::Ident(binding) = &*argument.pat
        {
            bindings.bind(&binding.ident, format!("__review_formal_{position}"));
        }
    }

    bindings.visit_block(&function.block);
    bindings.render(shapes::function_range(declaration.source, function))
}

struct Bindings<'a> {
    source: &'a str,
    names: BTreeMap<String, String>,
    replacements: BTreeMap<(u32, u32), String>,
    next: usize,
}

impl Bindings<'_> {
    fn render(self, range: ByteSpan) -> Result<String, String> {
        let original = source_evidence::byte_range(self.source, range)?;
        let mut output = String::new();
        let mut position = range.start as usize;
        for ((start, end), replacement) in self.replacements {
            output.push_str(&self.source[position..start as usize]);
            output.push_str(&replacement);
            position = end as usize;
        }

        output.push_str(&original[position - range.start as usize..]);
        canonical(&output)
    }

    fn bind(&mut self, identifier: &syn::Ident, name: String) {
        self.names.insert(identifier.to_string(), name.clone());
        self.replace(identifier, name);
    }

    fn replace(&mut self, identifier: &syn::Ident, name: String) {
        let range = shapes::span(self.source, identifier.span());
        self.replacements.insert((range.start, range.end), name);
    }
}

impl<'ast> Visit<'ast> for Bindings<'_> {
    fn visit_pat_ident(&mut self, binding: &'ast syn::PatIdent) {
        let name = format!("__review_binding_{}", self.next);
        self.next += 1;
        self.bind(&binding.ident, name);
        syn::visit::visit_pat_ident(self, binding);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
        {
            let identifier = &path.path.segments[0].ident;
            if let Some(name) = self.names.get(&identifier.to_string()) {
                self.replace(identifier, name.clone());
            }
        }
        syn::visit::visit_expr_path(self, path);
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let names = self.names.clone();
        syn::visit::visit_arm(self, arm);
        self.names = names;
    }
}

fn canonical(source: &str) -> Result<String, String> {
    proc_macro2::TokenStream::from_str(source)
        .map(|tokens| tokens.to_string())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer_facts::Availability;

    const DISPATCH: &str = "struct A; struct B; impl A {fn one(&self)->u32 {1}} impl B {fn two(&self)->u32 {2}} enum E {A(A),B(B)} fn caller(value:&E)->u32 {match value {E::A(left)=>left.one(),E::B(right)=>right.two()}}";
    const WRAPPER: &str = "struct A; impl A {fn one(&self)->u32 {1} fn two(&self)->u32 {2}} fn caller(value:&A)->(u32,u32) {(value.one(),value.two())}";

    fn observed_dispatch(source: &str) -> Dispatch {
        let modules = shapes::parse(source)
            .map_err(|failure| failure.reason)
            .unwrap();
        let (module_index, module) = modules
            .iter()
            .enumerate()
            .find(|(_, module)| {
                module
                    .functions
                    .iter()
                    .any(|function| function.sig.ident == "caller")
            })
            .unwrap();
        let function = module
            .functions
            .iter()
            .find(|function| function.sig.ident == "caller")
            .unwrap();
        let Availability::Known(Some(candidate)) =
            shapes::evaluate(module, function, source).availability
        else {
            panic!("positive dispatch fixture");
        };
        dispatch(
            CheckedDeclaration {
                module,
                function,
                source,
                owner_index: 0,
                module_index,
            },
            &candidate,
        )
        .unwrap()
    }

    fn observed_capability(source: &str) -> Capability {
        let modules = shapes::parse(source)
            .map_err(|failure| failure.reason)
            .unwrap();
        let module = &modules[0];
        let function = module
            .functions
            .iter()
            .find(|function| function.sig.ident == "caller")
            .unwrap();
        let declaration = || CheckedDeclaration {
            module,
            function,
            source,
            owner_index: 0,
            module_index: 0,
        };
        let Availability::Known(Some(wrapper)) =
            super::super::capabilities::evaluate(declaration()).availability
        else {
            panic!("positive wrapper fixture");
        };
        capability(declaration(), &wrapper).unwrap()
    }

    #[test]
    fn original_caller_tokens_ignore_comments_positions_and_scoped_binder_names() {
        let original = observed_dispatch(DISPATCH);
        let renamed = DISPATCH
            .replace("value", "input")
            .replace("left", "item")
            .replace("right", "item");
        let moved = observed_dispatch(&format!(
            "// 🦀\r\n{}",
            renamed.replace("match input", "match /* comment */ input")
        ));
        assert_eq!(original.caller.source, moved.caller.source);
        assert_eq!(original.enumeration.source, moved.enumeration.source);
        assert_eq!(original.caller.qualified_name, "crate::caller");
        assert_eq!(original.arms[0].1.method.qualified_name, "crate::A::one");
        assert_eq!(original.arms[1].1.method.qualified_name, "crate::B::two");
    }

    #[test]
    fn selected_method_headers_exclude_bodies_but_preserve_names_and_order() {
        let original = observed_capability(WRAPPER);
        let edited = observed_capability(&WRAPPER.replace("{1}", "{99}").replace("value", "input"));
        assert_eq!(original.caller.source, edited.caller.source);
        assert_eq!(
            original.selections[0].method.source,
            edited.selections[0].method.source
        );
        let reversed = observed_capability(
            &WRAPPER.replace("(value.one(),value.two())", "(value.two(),value.one())"),
        );
        assert_ne!(original.caller.source, reversed.caller.source);
        assert_eq!(
            original.selections[0].method.qualified_name,
            reversed.selections[1].method.qualified_name
        );
    }

    #[test]
    fn full_nominal_declarations_and_inline_module_paths_remain_relevant() {
        let original = observed_dispatch(DISPATCH);
        let field = observed_dispatch(&DISPATCH.replace("struct A;", "struct A { field: u32 }"));
        assert_ne!(
            original.arms[0].1.owner.source,
            field.arms[0].1.owner.source
        );
        let nested = observed_dispatch(&format!("mod outer {{mod inner {{{DISPATCH}}}}}"));
        assert_eq!(nested.caller.qualified_name, "crate::outer::inner::caller");
        assert_eq!(
            nested.arms[0].1.owner.qualified_name,
            "crate::outer::inner::A"
        );
    }
}
