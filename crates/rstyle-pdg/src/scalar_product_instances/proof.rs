//! Exact instance/source cleanup evidence, registered under selected::no_drop.
//! This receipt grants neither Copy nor constructor, pattern or value permission.
use super::*;
use crate::{
    concrete_types::source_proof::{self, PathResolution},
    nominal_boundary::resolution::{Resolution, builtin_options},
    scalar_product_instances::{Argument, Observation as Instance},
};

#[path = "proof/source.rs"]
pub(super) mod source;
#[cfg(test)]
#[path = "proof/tests.rs"]
mod tests;

pub(crate) struct CheckedScalarProductInstance {
    instance: Instance,
    owner: copy_products::Owner,
    context: copy_products::SourceContext,
    method: Option<source::Method>,
}

pub(in crate::nominal_boundary::resolution) fn capture_checked(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    instance: &Instance,
) -> Result<CheckedScalarProductInstance, String> {
    instance.check_shape()?;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    verify_interface(&interface, instance)?;
    let environment = &resolution.environment;
    environment.check_input_owner(facts)?;
    crate::declaration_origins::verify_in(&interface, origins, environment)?;
    let method = if crate::scalar_product_instances::forwarders::candidate(facts) {
        crate::scalar_product_instances::forwarders::source_shape(facts)?;
        None
    } else {
        Some(source::verify_method(
            facts,
            environment,
            instance.argument,
        )?)
    };
    verify_declaration(facts, resolution, origins, instance)?;
    let owner = match &method {
        Some(method) => {
            copy_products::Owner::capture_at(facts, environment, origins, method.namespace.clone())?
        }
        None => copy_products::Owner::capture(facts, environment, origins)?,
    };

    source::unshadowed_byte(facts, environment, owner.namespace())?;

    Ok(CheckedScalarProductInstance {
        instance: instance.clone(),
        owner,
        context: copy_products::SourceContext::capture(environment),
        method,
    })
}

impl CheckedScalarProductInstance {
    pub(super) fn output_source(
        instance: &Instance,
        owner: copy_products::Owner,
        environment: &Environment,
    ) -> Self {
        Self {
            instance: instance.clone(),
            owner,
            context: copy_products::SourceContext::capture(environment),
            method: None,
        }
    }

    pub(crate) fn implementation(&self) -> Option<crate::ByteSpan> {
        self.method.as_ref().map(|method| method.implementation)
    }

    pub(crate) fn instance(&self) -> &Instance {
        &self.instance
    }

    pub(crate) fn digits(&self) -> Result<u8, String> {
        self.method
            .as_ref()
            .map(source::Method::digits)
            .ok_or_else(|| "Forwarding source does not own a digit parser implementation".into())
    }

    pub(in crate::nominal_boundary::resolution) fn verify_namespace(
        &self,
        namespace: &[String],
    ) -> Result<(), String> {
        if self.owner.namespace() != namespace {
            return Err("Scalar product source requires its exact checked method namespace".into());
        }
        Ok(())
    }

    pub(in crate::nominal_boundary::resolution) fn verify_owner(
        &self,
        facts: &HirFnFacts,
        environment: &Environment,
        instance: &Instance,
    ) -> Result<(), String> {
        if self.instance != *instance {
            return Err(
                "Scalar product receipt belongs to another exact instance/source environment"
                    .into(),
            );
        }

        self.checked_namespace(facts, environment).map(|_| ())
    }

    pub(in crate::nominal_boundary::resolution) fn checked_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
    ) -> Result<&'a [String], String> {
        self.context.verify(facts, environment, &self.owner)?;
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        verify_interface(&interface, &self.instance)?;

        Ok(self.owner.namespace())
    }
}

pub(super) fn verify_interface(
    interface: &InterfaceObservations,
    instance: &Instance,
) -> Result<(), String> {
    interface.validate()?;
    let Observation::Captured {
        ty: TypeTree::Option(payload),
    } = &interface.result
    else {
        return Err("Digit product source requires a complete Option instance result".into());
    };
    if !matches!(payload.as_ref(), TypeTree::ScalarProductInstance(observed) if observed.as_ref() == instance)
    {
        return Err("Digit product source result differs from its exact scalar instance".into());
    }
    for observation in &interface.parameters {
        if let Observation::Captured { ty } = observation {
            reject_other_instances(ty, instance)?;
        }
    }

    Ok(())
}

fn reject_other_instances(tree: &TypeTree, instance: &Instance) -> Result<(), String> {
    if let TypeTree::ScalarProductInstance(observed) = tree
        && observed.as_ref() != instance
    {
        return Err("Digit product source cannot combine different scalar instances".into());
    }
    if let TypeTree::Shared(inner)
    | TypeTree::Slice(inner)
    | TypeTree::Option(inner)
    | TypeTree::Array { element: inner, .. } = tree
    {
        return reject_other_instances(inner, instance);
    }
    for child in tree
        .nominal_fields()
        .map(|field| &field.ty)
        .chain(tree.components())
    {
        reject_other_instances(child, instance)?;
    }

    Ok(())
}

pub(super) fn verify_declaration(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    instance: &Instance,
) -> Result<(), String> {
    let checker = product_fields::Proof::reference(facts, resolution, origins);
    let tree = TypeTree::ScalarProductInstance(Box::new(instance.clone()));
    let key = checker.declaration(&tree)?;
    let context = source::DeclarationContext {
        facts,
        resolution,
        origins,
    };
    source::verify_declaration(&context, instance, &key)?;
    verify_fields(facts, resolution, instance, &key)?;

    checker.closure(&key)
}

/// Exact declaration-scoped substitution only; grants no cleanup/value authority.
pub(in crate::nominal_boundary::resolution) fn verify_fields(
    facts: &HirFnFacts,
    resolution: &Resolution,
    instance: &Instance,
    key: &Namespace,
) -> Result<(), String> {
    instance.check_shape()?;
    if resolution.trees.get(key)
        != Some(&TypeTree::ScalarProductInstance(Box::new(instance.clone())))
    {
        return Err(
            "Scalar product field proof requires its complete exact registered instance".into(),
        );
    }
    let item: syn::ItemStruct =
        syn::parse_str(&instance.source).map_err(|error| error.to_string())?;
    let Some(syn::GenericParam::Type(parameter)) = item.generics.params.iter().nth(1) else {
        return Err("Missing scalar product original substitution parameter".into());
    };
    let (_, namespace) = key
        .split_last()
        .ok_or("Missing scalar product declaration namespace")?;
    let namespace = namespace.to_vec();
    let scope = Substitution {
        scope: builtin_options::DeclarationScope {
            resolution,
            namespace: &namespace,
            facts,
        },
        parameter: &parameter.ident,
        argument: &instance.fields[1].ty,
    };
    for field in &instance.fields {
        source_proof::verify_top_spelling(&field.declaration, &field.ty, &scope)?;
    }

    Ok(())
}

struct Substitution<'a> {
    scope: builtin_options::DeclarationScope<'a>,
    parameter: &'a syn::Ident,
    argument: &'a TypeTree,
}

impl PathResolution for Substitution<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        if path.qself.is_none() && path.path.is_ident(self.parameter) {
            return Ok(self.argument.clone());
        }
        self.scope.resolve(path)
    }
}
