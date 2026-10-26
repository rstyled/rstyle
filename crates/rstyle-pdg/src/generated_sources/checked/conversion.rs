//! Exact generated TryFrom source association; no cleanup, lexical or call permission.
use super::*;
use crate::{
    concrete_types::{InterfaceObservations, Observation as TypeObservation, TypeTree},
    tryfrom_calls::{self, model},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    status: String,
    observation: model::Observation,
}

pub(super) fn verify(
    input: &Input<'_>,
    generated: &Observation,
    expansion: &source::CheckedExpansion,
    item: &syn::ItemImpl,
) -> Result<String, String> {
    let raw = input
        .facts
        .build
        .frontend_options
        .get("rstyle.rust.generated-conversion-member")
        .ok_or("Missing generated conversion owner observation")?;
    let outcome = Outcome::deserialize(raw).map_err(|error| error.to_string())?;
    if outcome.status != "observed"
        || outcome.observation.identity != "rstyle.rust.generated-conversion-member-observation/1"
        || outcome.observation.admission_authority
    {
        return Err("Generated conversion observation is incomplete or claims authority".into());
    }

    let check = Checker {
        facts: input.facts,
        observed: &outcome.observation,
        generated,
        expansion,
    };
    check.verify(item)
}

struct Checker<'a> {
    facts: &'a HirFnFacts,
    observed: &'a model::Observation,
    generated: &'a Observation,
    expansion: &'a source::CheckedExpansion,
}

impl Checker<'_> {
    fn verify(&self, item: &syn::ItemImpl) -> Result<String, String> {
        let method = item
            .items
            .iter()
            .find_map(|member| match member {
                syn::ImplItem::Fn(method) => Some(method),
                _ => None,
            })
            .ok_or("Missing generated conversion method")?;

        let input = tryfrom_calls::input::captured(self.facts)?;
        let paths = tryfrom_calls::syntax::implementation(item, method, input)?;
        let (success, error) = self.interface(input)?;
        self.nominal_paths(&paths, success, error)?;
        self.definitions()?;
        self.sources(item, method)?;
        for signature in [
            &self.observed.requested_signature,
            &self.observed.selected_signature,
        ] {
            tryfrom_calls::verify_signature(
                self.facts,
                signature,
                &[success.clone(), error.clone()],
                &tryfrom_calls::RegistryAuthority,
            )?;
        }
        let owner = plain_nominal(&paths.self_type)?;
        let prefix = self.expansion.namespace().join("::");
        let owner = if prefix.is_empty() {
            owner
        } else {
            format!("{prefix}::{owner}")
        };
        let expected = format!("<{owner} as core::convert::TryFrom<{input}>>");
        if self.observed.implementation.path.replace("std::", "core::") != expected {
            return Err(
                "Generated conversion implementation path differs from exact source owner".into(),
            );
        }
        Ok(self.observed.implementation.path.clone())
    }

    fn interface(&self, input: &str) -> Result<(&model::Structural, &model::Structural), String> {
        let interface = self
            .facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        let TypeObservation::Captured {
            ty: TypeTree::Result { ok, err },
        } = &interface.result
        else {
            return Err("Generated conversion lacks complete Result interface".into());
        };
        if !matches!(interface.parameters.as_slice(), [TypeObservation::Captured { ty: TypeTree::Scalar(name) }] if name == input)
        {
            return Err("Generated conversion interface changes its exact unsigned input".into());
        }

        let result = crate::data_types::TypeToken::Result {
            ok: Box::new(crate::data_types::TypeToken::from_tree(ok)),
            err: Box::new(crate::data_types::TypeToken::from_tree(err)),
        };
        if self.facts.signature.return_ty.normalized.as_deref() != Some(result.text()?.as_str()) {
            return Err(
                "Generated conversion return capture differs from complete interface".into(),
            );
        }
        let [self_argument, scalar] = self.observed.trait_arguments.as_slice() else {
            return Err("Generated conversion requires exact Self and input substitutions".into());
        };
        if !matches!(scalar.known()?, model::Structural::Builtin { name } if name == input) {
            return Err("Generated conversion trait input differs from captured builtin".into());
        }
        let success = self_argument.known()?;
        verify_nominal(success, ok)?;
        let associated = self
            .observed
            .associated_error
            .as_ref()
            .ok_or("Missing actual associated Error")?;
        let error = tryfrom_calls::endpoint(&associated.declared_type)?;
        verify_nominal(error, err)?;
        if associated.declared_type != associated.normalized_projection {
            return Err("Generated associated Error differs from normalized projection".into());
        }
        Ok((success, error))
    }

    fn nominal_paths(
        &self,
        paths: &tryfrom_calls::syntax::Paths,
        success: &model::Structural,
        error: &model::Structural,
    ) -> Result<(), String> {
        let owner = plain_nominal(&paths.self_type)?;
        let associated = plain_nominal(&paths.error_type)?;

        for (name, structural) in [(&owner, success), (&associated, error)] {
            let model::Structural::Adt { definition, .. } = structural else {
                return Err(
                    "Generated conversion payload lacks an exact nominal definition".into(),
                );
            };
            tryfrom_calls::verify_local(self.facts, definition)?;
            let expected = self
                .expansion
                .namespace()
                .iter()
                .cloned()
                .chain([name.clone()])
                .collect::<Vec<_>>()
                .join("::");
            if definition.path != expected {
                return Err("Generated conversion source names another nominal payload".into());
            }
        }
        let (ok, err) = tryfrom_calls::syntax::result_arguments(&paths.result_type)?;
        if plain_nominal(ok)? != owner || plain_nominal(err)? != associated {
            return Err(
                "Generated conversion source Result changes its associated payloads".into(),
            );
        }
        trait_path(&paths.trait_path)
    }

    fn definitions(&self) -> Result<(), String> {
        let observed = self.observed;
        let associated = observed
            .associated_error
            .as_ref()
            .ok_or("Missing associated Error")?;
        for definition in [
            &observed.method,
            &observed.implementation,
            &associated.implementation_member,
        ] {
            tryfrom_calls::verify_local(self.facts, definition)?;
        }
        for (role, definition) in [
            (tryfrom_calls::CoreRole::Trait, &observed.trait_definition),
            (tryfrom_calls::CoreRole::Method, &observed.trait_method),
            (
                tryfrom_calls::CoreRole::ErrorMember,
                &associated.trait_member,
            ),
        ] {
            tryfrom_calls::verify_core(
                self.facts,
                role,
                definition,
                &tryfrom_calls::RegistryAuthority,
            )?;
        }

        crate::source_evidence::require_definition(
            self.facts,
            &observed.method.path,
            &observed.method.definition_identity,
        )?;
        if !same(&observed.method, &self.generated.owner)
            || !same(&observed.implementation, &self.generated.implementation)
            || observed.method != observed.selected
            || !observed.selected_arguments.is_empty()
            || observed.method_header.is_const
            || observed.method_header.is_async
        {
            return Err(
                "Generated conversion selection differs from exact generated owner/header".into(),
            );
        }
        Ok(())
    }

    fn sources(&self, item: &syn::ItemImpl, method: &syn::ImplItemFn) -> Result<(), String> {
        let observed = self.observed;
        let associated = observed
            .associated_error
            .as_ref()
            .ok_or("Missing associated Error")?;
        if observed.implementation_source != self.expansion.source_span(item.span())
            || observed.method_source != self.expansion.source_span(method.span())
            || observed.members.len() != item.items.len()
        {
            return Err("Generated conversion source anchors or complete roster disagree".into());
        }

        for (syntax, actual) in item.items.iter().zip(&observed.members) {
            let (definition, trait_member) = match syntax {
                syn::ImplItem::Fn(_) => (&observed.method, &observed.trait_method),
                syn::ImplItem::Type(_) => {
                    (&associated.implementation_member, &associated.trait_member)
                }
                _ => return Err("Unsupported generated conversion member".into()),
            };
            let source = self.expansion.source_span(syntax.span());
            if actual.definition != *definition
                || actual.trait_member.as_ref() != Some(trait_member)
                || actual.source != source
                || matches!(syntax, syn::ImplItem::Type(_)) && associated.source != source
            {
                return Err(
                    "Generated conversion roster changes its actual associated member/source"
                        .into(),
                );
            }
        }
        Ok(())
    }
}

fn plain_nominal(path: &syn::TypePath) -> Result<String, String> {
    path.path
        .get_ident()
        .filter(|_| path.qself.is_none())
        .map(ToString::to_string)
        .ok_or_else(|| "Generated conversion requires plain same-module nominal source".into())
}

fn verify_nominal(structural: &model::Structural, tree: &TypeTree) -> Result<(), String> {
    let nominal = tree
        .nominal_identity()
        .ok_or("Generated conversion interface lacks nominal payload")?;
    let definition = tryfrom_calls::verify_nominal(structural, nominal)?;
    if tree
        .generated_parts()
        .is_some_and(|(_, actual, ..)| !same(&definition, actual))
    {
        return Err(
            "Generated conversion interface has another generated definition identity".into(),
        );
    }
    Ok(())
}

fn same(actual: &model::Definition, expected: &Definition) -> bool {
    actual.path == expected.path
        && actual.definition_identity == expected.definition_identity
        && actual.crate_name == expected.crate_name
        && actual.stable_crate_id == expected.stable_crate_id
        && actual.local == expected.local
}

fn trait_path(path: &syn::Path) -> Result<(), String> {
    let names: Vec<_> = path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    if !matches!(
        names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["TryFrom"] | ["core", "convert", "TryFrom"] | ["std", "convert", "TryFrom"]
    ) {
        return Err("Generated conversion source uses an unreviewed trait path".into());
    }
    Ok(())
}
