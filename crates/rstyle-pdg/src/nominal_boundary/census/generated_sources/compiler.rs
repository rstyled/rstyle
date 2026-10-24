//! Normalize compiler header/macro spans by exact original AST boundaries.
use super::*;
use crate::nominal_boundary::census::original_sites::site as anchor;

pub struct CompilerInput<'a> {
    pub census: &'a Census,
    pub cfgs: &'a [String],
    pub definition_header: &'a FileSpan,
    pub invocation_macro: &'a FileSpan,
}

pub fn verify_compiler(input: CompilerInput<'_>) -> Result<CheckedExpansion, String> {
    let modules = super::super::verify_with_module_lints(input.census, input.cfgs, true)?;
    verify_compiler_modules(input, &modules)
}

pub(crate) fn verify_compiler_in(
    input: CompilerInput<'_>,
    environment: &crate::nominal_boundary::resolution::Environment,
) -> Result<CheckedExpansion, String> {
    if !std::ptr::eq(input.census, environment.census()) {
        return Err("Compiler source context belongs to a different census".into());
    }
    match environment.generated_source_modules(input.cfgs) {
        Some(modules) => verify_compiler_modules(input, modules),
        None => verify_compiler(input),
    }
}

fn verify_compiler_modules(
    input: CompilerInput<'_>,
    modules: &[Module],
) -> Result<CheckedExpansion, String> {
    let sites = sites(&input, modules)?;
    verify_modules(
        Input {
            census: input.census,
            cfgs: input.cfgs,
            namespace: &sites.namespace,
            definition: &sites.definition,
            invocation: &sites.invocation,
        },
        modules,
        true,
    )
}

struct Sites {
    definition: FileSpan,
    invocation: FileSpan,
    namespace: Vec<String>,
}

fn sites(input: &CompilerInput<'_>, modules: &[Module]) -> Result<Sites, String> {
    let mut definitions = Vec::new();
    let mut invocations = Vec::new();
    for module in modules {
        let file = input
            .census
            .files
            .iter()
            .find(|file| file.path == module.path)
            .ok_or("Missing original compiler-anchor buffer")?;
        for item in &module.items {
            let syn::Item::Macro(item) = item else {
                continue;
            };
            if item.mac.path.is_ident("macro_rules") {
                if definition_header(item)
                    .is_some_and(|span| anchor(file, span) == *input.definition_header)
                {
                    definitions.push(anchor(file, item.span()));
                }
            } else if anchor(file, item.mac.span()) == *input.invocation_macro {
                invocations.push((anchor(file, item.span()), module.namespace.clone()));
            }
        }
    }

    if definitions.len() != 1 || invocations.len() != 1 {
        return Err("Compiler macro anchors lack exact unique original AST boundaries".into());
    }
    let (invocation, namespace) = invocations.remove(0);
    Ok(Sites {
        definition: definitions.remove(0),
        invocation,
        namespace,
    })
}

fn definition_header(item: &syn::ItemMacro) -> Option<proc_macro2::Span> {
    item.mac.path.span().join(item.ident.as_ref()?.span())
}
