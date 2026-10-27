//! Exact original nongeneric inherent owner and complete S0 trait clause roster.
use super::*;
use crate::nominal_boundary::census::Census;
use crate::source_evidence::original_span;
use std::sync::Arc;

pub(super) fn verify(facts: &HirFnFacts, observed: &Value) -> Result<()> {
    if observed["status"] != "observed" {
        return Err("Product instance is not observed".into());
    }
    let census = facts.build.frontend_options.data::<Arc<Census>>()?;

    crate::nominal_boundary::census::check_budget(&census)?;
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        .ok_or("Product original source file missing")?;
    let files: Vec<_> = census
        .files
        .iter()
        .filter(|item| item.path == std::path::Path::new(file))
        .collect();
    let [source] = files.as_slice() else {
        return Err("Product original source file absent or ambiguous".into());
    };
    original_bytes(facts, &source.bytes)?;
    OwnerSource {
        facts,
        observed,
        file,
        bytes: &source.bytes,
    }
    .verify()
}

fn original_bytes(facts: &HirFnFacts, bytes: &str) -> Result<()> {
    if bytes.get(facts.fn_span.start as usize..facts.fn_span.end as usize)
        != Some(facts.source.bytes.as_ref())
    {
        return Err("Product owner source differs from canonical original bytes".into());
    }
    Ok(())
}

struct OwnerSource<'a> {
    facts: &'a HirFnFacts,
    observed: &'a Value,
    file: &'a str,
    bytes: &'a str,
}

impl OwnerSource<'_> {
    fn verify(&self) -> Result<()> {
        let syntax = syn::parse_file(self.bytes).map_err(|e| e.to_string())?;

        let mut candidates = Vec::new();
        find(&syntax.items, self.bytes, self.facts, &mut candidates);
        let [(implementation, function)] = candidates.as_slice() else {
            return Err("Product original impl method is absent or ambiguous".into());
        };
        let owners = self.observed["owners"]
            .as_array()
            .ok_or("Product owners missing")?;
        let [parent, method] = owners.as_slice() else {
            return Err("Product requires original impl and method owners".into());
        };
        Self::header(implementation, parent, method)?;
        self.identities(parent, method)?;
        self.anchors(implementation, function, parent)?;
        self.parameter(function, method)
    }

    fn header(implementation: &syn::ItemImpl, parent: &Value, method: &Value) -> Result<()> {
        if implementation.trait_.is_some()
            || implementation.unsafety.is_some()
            || implementation.defaultness.is_some()
            || !implementation.generics.params.is_empty()
            || implementation.generics.where_clause.is_some()
            || !matches!(&*implementation.self_ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("Match"))
            || parent["parent"] != Value::Null
            || method["parent"] != parent["definition"]
            || parent["parameters"] != serde_json::json!([])
            || parent["clauses"] != serde_json::json!([])
            || parent["source_predicates"] != serde_json::json!([])
        {
            return Err("Product original inherent owner invents parameters or predicates".into());
        }
        Ok(())
    }

    fn identities(&self, parent: &Value, method: &Value) -> Result<()> {
        let facts = self.facts;

        definition(facts, &parent["definition"], "util::search::Match")?;
        definition(facts, &method["definition"], &facts.name)?;
        if method["definition"]["definition_identity"]
            != facts
                .build
                .frontend_options
                .get("definition_identity")
                .cloned()
                .ok_or("Missing canonical product definition identity")?
        {
            return Err("Product method observation changes its canonical owner identity".into());
        }
        Ok(())
    }

    fn anchors(
        &self,
        implementation: &syn::ItemImpl,
        function: &syn::ImplItemFn,
        parent: &Value,
    ) -> Result<()> {
        let facts = self.facts;

        let file = self.file;
        let method = &self.observed["owners"][1];
        anchor(
            &parent["declaration"],
            file,
            crate::ByteSpan {
                start: original_span(self.bytes, implementation.impl_token.span()).start,
                end: original_span(self.bytes, implementation.self_ty.span()).end,
            },
        )?;
        anchor(
            &method["declaration"],
            file,
            crate::ByteSpan {
                start: facts.fn_span.start,
                end: original_span(self.bytes, function.sig.span()).end,
            },
        )?;
        Ok(())
    }

    fn parameter(&self, function: &syn::ImplItemFn, method: &Value) -> Result<()> {
        let facts = self.facts;

        let file = self.file;
        let generic = &function.sig.generics.params[0];
        let syn::GenericParam::Type(generic) = generic else {
            return Err("Product original S missing".into());
        };
        let parameters = method["parameters"]
            .as_array()
            .ok_or("Product parameter roster missing")?;
        let [parameter] = parameters.as_slice() else {
            return Err("Product S parameter omitted or duplicated".into());
        };
        if parameter["index"] != 0 || parameter["name"] != "S" {
            return Err("Product S parameter index differs".into());
        }
        definition(
            facts,
            &parameter["definition"],
            &format!("{}::S", facts.name),
        )?;
        let generic_span = original_span(self.bytes, generic.ident.span());
        anchor(&parameter["declaration"], file, generic_span)?;
        anchor(&parameter["name_anchor"], file, generic_span)?;
        let bound = &generic.bounds[0];
        let bound_span = original_span(self.bytes, bound.span());
        let predicates = method["source_predicates"]
            .as_array()
            .ok_or("Product source predicates missing")?;
        let [predicate] = predicates.as_slice() else {
            return Err("Product source predicate roster differs".into());
        };
        anchor(
            predicate,
            file,
            crate::ByteSpan {
                start: generic_span.end,
                end: bound_span.end,
            },
        )?;
        self.clauses(generic_span, bound_span)
    }
}

fn find<'a>(
    items: &'a [syn::Item],
    bytes: &str,
    facts: &HirFnFacts,
    output: &mut Vec<(&'a syn::ItemImpl, &'a syn::ImplItemFn)>,
) {
    for item in items {
        match item {
            syn::Item::Impl(implementation) => {
                for item in &implementation.items {
                    if let syn::ImplItem::Fn(function) = item {
                        let start = original_span(bytes, function.vis.span()).start;
                        let end = original_span(bytes, function.block.span()).end;
                        if (crate::ByteSpan { start, end }) == facts.fn_span {
                            output.push((implementation, function));
                        }
                    }
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    find(items, bytes, facts, output);
                }
            }
            _ => {}
        }
    }
}

fn anchor(value: &Value, file: &str, span: crate::ByteSpan) -> Result<()> {
    if *value != serde_json::json!({"source_file":file,"span":span}) {
        return Err("Product original source anchor differs".into());
    }
    Ok(())
}

fn definition(facts: &HirFnFacts, value: &Value, path: &str) -> Result<()> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Product compiled crate missing")?;

    if value["path"] != path
        || value["local"] != true
        || value["crate"] != compiled["crate_name"]
        || value["stable_crate_id"] != compiled["stable_crate_id"]
        || value["definition_identity"]
            .as_str()
            .is_none_or(str::is_empty)
        || value.get("crate_hash").is_some()
    {
        return Err("Product original local definition changes crate/owner identity".into());
    }
    Ok(())
}

impl OwnerSource<'_> {
    fn clauses(&self, generic: crate::ByteSpan, bound: crate::ByteSpan) -> Result<()> {
        let observed = self.observed;

        let method = &observed["owners"][1];
        let file = self.file;
        let clauses = method["clauses"]
            .as_array()
            .ok_or("Product method clauses missing")?;
        let [sized, into] = clauses.as_slice() else {
            return Err("Product requires complete Sized and Into clauses".into());
        };
        sized_authority(self.facts, &sized["trait_definition"])?;
        let parameter = serde_json::json!({"kind":"parameter","index":0,"name":"S"});
        let conversion = &observed["conversions"][0];
        if sized["positive"] != true
            || into["positive"] != true
            || sized["trait_definition"]["path"] != "core::marker::Sized"
            || sized["trait_definition"]["crate"] != "core"
            || sized["trait_definition"]["local"] != false
            || sized["trait_definition"]["crate_hash"]
                != conversion["into"]["requested_trait"]["crate_hash"]
            || sized["trait_definition"]["stable_crate_id"]
                != conversion["into"]["requested_trait"]["stable_crate_id"]
            || sized["trait_definition"]["definition_identity"]
                .as_str()
                .is_none_or(str::is_empty)
            || into["trait_definition"] != conversion["into"]["requested_trait"]
            || sized["original_arguments"] != serde_json::json!([parameter])
            || sized["substituted_arguments"] != observed["arguments"]
            || into["substituted_arguments"] != conversion["into"]["requested_arguments"]
        {
            return Err("Product original trait clauses or concrete substitutions differ".into());
        }
        let span = &conversion["result_type"][0]["identity"];
        if into["original_arguments"] != serde_json::json!([parameter, span]) {
            return Err("Product Into bound changes its original S-to-Span roles".into());
        }
        anchor(&sized["anchor"], file, generic)?;
        anchor(&into["anchor"], file, bound)
    }
}

fn sized_authority(facts: &HirFnFacts, value: &Value) -> Result<()> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned product Sized trait compiler/target")?;

    let string = |key: &str| {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("Product Sized trait missing {key}"))
    };
    let callee = crate::data_library::Callee {
        path: string("path")?,
        crate_name: string("crate")?,
        crate_hash: string("crate_hash")?,
        definition_hash: string("definition_identity")?,
        implementation: None,
    };
    if !registry.exact_core_definition(&callee, "rust.product-conversion.sized-trait") {
        return Err("Product Sized trait lacks its exact reviewed definition".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn checked_clauses(facts: &HirFnFacts, value: &Value) -> Result<()> {
        let method = &value["owners"][1];

        let file = method["declaration"]["source_file"].as_str().unwrap();
        let generic =
            serde_json::from_value(method["parameters"][0]["name_anchor"]["span"].clone()).unwrap();
        let bound = serde_json::from_value(method["clauses"][1]["anchor"]["span"].clone()).unwrap();
        OwnerSource {
            facts,
            observed: value,
            file,
            bytes: "",
        }
        .clauses(generic, bound)
    }

    #[test]
    fn original_complete_trait_clauses_require_pinned_sized_and_exact_into() {
        let (facts, instance) = super::super::tests::fixture();

        checked_clauses(&facts, &instance).unwrap();
        for pointer in [
            "/owners/1/clauses/0/trait_definition/definition_identity",
            "/owners/1/clauses/0/trait_definition/crate_hash",
            "/owners/1/clauses/0/trait_definition/stable_crate_id",
            "/owners/1/clauses/0/positive",
            "/owners/1/clauses/1/positive",
            "/owners/1/clauses/1/trait_definition/definition_identity",
            "/owners/1/clauses/1/original_arguments/0/index",
            "/owners/1/clauses/1/original_arguments/1/definition/definition_identity",
            "/owners/1/clauses/1/substituted_arguments/0/identity/definition/definition_identity",
            "/owners/1/clauses/0/anchor/span/end",
        ] {
            let mut changed = instance.clone();
            *changed.pointer_mut(pointer).unwrap() = json!("forged");
            assert!(checked_clauses(&facts, &changed).is_err(), "{pointer}");
        }
    }
}
