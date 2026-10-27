//! One source-checked external bang macro, retained by its exact consumer import.
use super::*;
use crate::{
    dependency_namespaces::capture::{Binding, Import, Site, UnitCapture},
    nominal_boundary::resolution::selected::no_drop::external_expression_macros::{self, Receipt},
};

use external_expression_macros::Review;
pub(super) use external_expression_macros::{CAPABILITY, CORRECTED_CAPABILITY};

pub(super) struct OwnedMacro {
    import: Site,
    receipt: Receipt,
}

impl Environment {
    pub(super) fn add_external_macro(
        &mut self,
        import: &Import,
        functions: &[HirFnFacts],
        corrected: bool,
    ) {
        // Discovery only: these spellings grant no source or artifact authority.
        if import.site.target_kind != "Mod"
            || import.site.target.path != "powerfmt::smart_display"
            || import.site.binding != Binding::Named("self".into())
        {
            return;
        }
        // Failed evidence retains the ordinary unresolved external-macro barrier.
        if let Ok(owned) = producer(import, functions, corrected)
            && crate::dependency_namespaces::proof::verify_site(
                &self.census,
                &self.modules,
                &owned.import,
            )
            .is_ok()
        {
            self.dependency_namespaces.external_macros.push(owned);
        }
    }

    fn external_import(&self, site: &Site) -> Result<(), String> {
        crate::dependency_namespaces::proof::verify_site(&self.census, &self.modules, site)
            .map_err(|error| error.to_string())?;
        let prefix = site
            .path
            .strip_suffix(&["self".into()])
            .ok_or("External macro requires its original module self import")?;
        let expected = ["powerfmt".to_owned(), "smart_display".to_owned()];
        if prefix != expected
            || self.foreign_namespace(&site.scope, prefix, "powerfmt")? != expected
            || site.target.path != "powerfmt::smart_display"
            || site.target.local
            || site.binding != Binding::Named("self".into())
            || site.target_kind != "Mod"
            || !self.external_binding_clear(&site.scope, "smart_display")
        {
            return Err("External macro module route is shadowed or differs".into());
        }

        if self
            .index
            .imports
            .get(&(site.scope.clone(), "smart_display".into()))
            != Some(&expected.to_vec())
        {
            return Err("External macro lacks its effective original module binding".into());
        }

        Ok(())
    }

    fn external_binding_clear(&self, scope: &[String], binding: &str) -> bool {
        let candidate: Vec<_> = scope.iter().cloned().chain([binding.into()]).collect();
        !self.index.declarations.contains_key(&candidate)
            && !self
                .modules
                .iter()
                .any(|module| module.namespace == candidate)
            && self.external_globs_clear(scope)
            && self
                .modules
                .iter()
                .filter(|module| scope.starts_with(&module.namespace))
                .all(|module| {
                    module.items.iter().all(|item| {
                        let name = builtin_options::type_name(item);
                        name.as_deref() != Some("powerfmt")
                            && (module.namespace != scope || name.as_deref() != Some(binding))
                    })
                })
    }

    fn external_globs_clear(&self, scope: &[String]) -> bool {
        let configuration = crate::conditional_source::Configuration::new(
            self.compiler_cfgs.as_deref().unwrap_or(&[]),
        );
        self.index
            .imports
            .iter()
            .all(|((namespace, name), target)| {
                if !scope.starts_with(namespace) || !name.starts_with("*::") {
                    return true;
                }
                self.anonymous_glob(namespace, target)
                    || self.external_generated_glob_clear(namespace, name)
                    || ["smart_display", "powerfmt"].iter().all(|binding| {
                        selected::glob_excludes(self, target, binding, &configuration).is_ok()
                    })
            })
    }

    fn external_generated_glob_clear(&self, namespace: &[String], key: &str) -> bool {
        // The index value is anchored (crate::convert becomes convert), whereas
        // the retained key preserves the original source route for the receipt.
        let Some(path) = key.strip_prefix("*::") else {
            return false;
        };
        let target = path.split("::").map(str::to_owned).collect::<Vec<_>>();
        let mut found = false;

        for module in self
            .modules
            .iter()
            .filter(|module| module.namespace == namespace)
        {
            for item in &module.items {
                if let syn::Item::Use(item) = item
                    && contains_glob(&item.tree, &[], &target)
                {
                    let Some(receipt) = self.generated_use(namespace, item) else {
                        return false;
                    };
                    if receipt
                        .names()
                        .any(|name| ["smart_display", "powerfmt"].contains(&name))
                    {
                        return false;
                    }
                    found = true;
                }
            }
        }

        found
    }

    /// The caller checks nested lexical shadows and original invocation origin,
    /// then exhaustively visits every returned value and option expression.
    pub(in crate::nominal_boundary::resolution) fn external_macro_expressions(
        &self,
        namespace: &[String],
        names: &[String],
        invocation: &syn::Macro,
    ) -> Result<Option<Vec<syn::Expr>>, String> {
        if names != ["smart_display", "padded_width_of"]
            || invocation.path.leading_colon.is_some()
            || invocation
                .path
                .segments
                .iter()
                .any(|part| !part.arguments.is_empty())
            || invocation
                .path
                .segments
                .iter()
                .map(|part| part.ident.to_string())
                .ne(names.iter().cloned())
        {
            return Ok(None);
        }

        let mut matching = self
            .dependency_namespaces
            .external_macros
            .iter()
            .filter(|owned| owned.import.scope == namespace);
        let Some(owned) = matching.next() else {
            return Ok(None);
        };

        if matching.next().is_some() {
            return Err("Ambiguous original external macro module ownership".into());
        }
        if owned.receipt.census().context.crate_name != owned.import.target.crate_name {
            return Err("External macro source owner differs from original module import".into());
        }
        self.external_import(&owned.import)?;

        owned
            .receipt
            .expressions(invocation, owned.receipt.definition())
            .map(Some)
    }
}

fn producer(
    import: &Import,
    functions: &[HirFnFacts],
    corrected: bool,
) -> Result<OwnedMacro, String> {
    let facts = functions.first().ok_or("Missing external macro producer")?;
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing original external macro compiled owner")?;

    for field in ["identity", "crate_name", "stable_crate_id"] {
        if compiled.get(field).is_none()
            || compiled.get(field) != import.compiled_dependency.get(field)
        {
            return Err("External macro compiled producer differs from associated import".into());
        }
    }

    let namespaces = crate::dependency_namespaces::unit_capture(functions)?
        .ok_or("Missing external macro producer namespaces")?;
    let census = facts.build.frontend_options.data::<Arc<census::Census>>()?;
    prove(
        import,
        Arc::clone(&census),
        &namespaces,
        Review {
            cfgs: &facts.build.cfgs,
            corrected,
        },
    )
}

fn prove(
    import: &Import,
    census: Arc<census::Census>,
    namespaces: &UnitCapture,
    review: Review<'_>,
) -> Result<OwnedMacro, String> {
    let exports: Vec<_> = import
        .exports
        .iter()
        .filter(|export| export.binding == Binding::Named("padded_width_of".into()))
        .collect();
    let [export] = exports.as_slice() else {
        return Err("Missing or ambiguous original padded-width macro export".into());
    };
    if import.site.target.definition_identity.is_empty()
        || import.site.target.crate_name != census.context.crate_name
        || import.site.target.stable_crate_id != export.target.stable_crate_id
        || import.site.target.crate_name != export.target.crate_name
        || !namespaces.declarations.iter().any(|declaration| {
            let owner = &declaration.site.owner;
            export.route.first().is_some_and(|route| {
                declaration.site.definition.definition_identity
                    == route.declaration.definition_identity
                    && declaration.site.definition.stable_crate_id
                        == route.declaration.stable_crate_id
                    && declaration.site.definition.crate_name == route.declaration.crate_name
            }) && owner.local
                && owner.path == "smart_display"
                && owner.definition_identity == import.site.target.definition_identity
                && owner.stable_crate_id == import.site.target.stable_crate_id
                && owner.crate_name == import.site.target.crate_name
        })
    {
        return Err("External macro export lacks its exact original module owner".into());
    }
    Ok(OwnedMacro {
        import: import.site.clone(),
        receipt: external_expression_macros::from_owner(census, namespaces, export, review)?,
    })
}

#[cfg(test)]
mod tests;
