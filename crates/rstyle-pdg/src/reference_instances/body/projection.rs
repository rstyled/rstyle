//! Exact instantiated node roster with source-backed nominal component shapes.
//! Opaque local compiler IDs are checked for consistency, not independently minted.
mod normalized;
use super::{
    Result, Role, Site,
    model::{Argument, Definition, Observation, Type},
    site,
};
use crate::reference_instances::view_types::{BorrowedViewType, ViewType};
use crate::{
    HirFnFacts,
    concrete_types::{InterfaceObservations, Nominal, Observation as Captured, TypeTree},
};
use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) fn verify(
    canonical: &HirFnFacts,
    record: &Observation,
    expected: &BTreeMap<Site, Role>,
    haystack: &str,
) -> Result<()> {
    let interface = canonical
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let nominals = Nominals::new(&interface)?;
    normalized::verify(canonical, expected, &nominals, haystack)?;
    let mut checked = Projection {
        canonical,
        haystack,
        nominals,
        identities: BTreeMap::new(),
    };
    let output = checked.argument(&record.projected_result, Role::Input)?;
    let Type::Adt { definition, .. } = output else {
        return Err("Constructor projection result is not the complete Input product".into());
    };
    let original = Definition::deserialize(&record.provenance["signature"]["output"]["definition"])
        .map_err(|error| error.to_string())?;
    if definition != &original {
        return Err(
            "Constructor projected result changes its original opaque nominal identity".into(),
        );
    }

    instance_arguments(&record.arguments, haystack)?;
    if record.nodes.len() != expected.len() {
        return Err("Constructor projection omits or invents value-node evidence".into());
    }
    let mut remaining = expected.clone();
    for node in &record.nodes {
        let role = remaining
            .remove(&site(&node.hir))
            .ok_or("Constructor projected node duplicates or invents an original HIR value")?;
        if node.kind != "value" {
            return Err("Constructor projection introduces an unproved function-item node".into());
        }
        checked.argument(&node.ty, role)?;
    }
    Ok(())
}

fn instance_arguments(arguments: &[Argument], haystack: &str) -> Result<()> {
    let [lifetime, concrete] = arguments else {
        return Err("Constructor projection generic substitution roster differs".into());
    };
    argument(lifetime, "lifetime", &Type::ErasedRegion)?;
    argument(concrete, "type", &Type::haystack(haystack)?)
}

fn argument(value: &Argument, kind: &str, expected: &Type) -> Result<()> {
    if value.kind != kind
        || !value.display_only
        || value.identity_status != "known"
        || value.identity != *expected
    {
        return Err(
            "Constructor projected argument differs from independently expected structural type"
                .into(),
        );
    }
    Ok(())
}

struct Projection<'a> {
    canonical: &'a HirFnFacts,
    haystack: &'a str,
    nominals: Nominals<'a>,
    identities: BTreeMap<String, Definition>,
}

impl Projection<'_> {
    fn argument<'a>(&mut self, values: &'a [Argument], role: Role) -> Result<&'a Type> {
        let [value] = values else {
            return Err("Constructor value requires exactly one complete projected type".into());
        };
        if value.kind != "type" || !value.display_only || value.identity_status != "known" {
            return Err(
                "Constructor projected value lacks complete structural type identity".into(),
            );
        }
        match role {
            Role::Input | Role::Span | Role::Anchored => self.nominal(&value.identity, role)?,
            _ => {
                let expected = match role {
                    Role::Formal => Type::haystack(self.haystack)?.shared(),
                    Role::Bytes => Type::bytes().shared(),
                    Role::Usize => Type::Builtin {
                        name: "usize".into(),
                    },
                    Role::Bool => Type::Builtin {
                        name: "bool".into(),
                    },
                    _ => return Err("Unknown constructor scalar projection role".into()),
                };
                argument(value, "type", &expected)?;
            }
        }
        Ok(&value.identity)
    }

    fn nominal(&mut self, ty: &Type, role: Role) -> Result<()> {
        let Type::Adt {
            definition,
            arguments,
        } = ty
        else {
            return Err("Constructor nominal value lost its structural ADT identity".into());
        };
        let (nominal, lifetimes) = self.nominals.get(role)?;
        let compiled = self
            .canonical
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            .ok_or("Constructor projection lacks canonical compiled owner")?;
        if definition.path != nominal.definition_path
            || !definition.local
            || definition.definition_identity.is_empty()
            || compiled
                .get("crate_name")
                .and_then(serde_json::Value::as_str)
                != Some(definition.crate_name.as_str())
            || compiled
                .get("stable_crate_id")
                .and_then(serde_json::Value::as_str)
                != Some(definition.stable_crate_id.as_str())
            || arguments.len() != lifetimes
            || arguments
                .iter()
                .any(|argument| *argument != Type::ErasedRegion)
        {
            return Err(
                "Constructor projected nominal type changes its canonical source component".into(),
            );
        }

        if self
            .identities
            .get(&definition.path)
            .is_some_and(|prior| prior != definition)
        {
            return Err(
                "Constructor projected nodes disagree on the same opaque nominal compiler identity"
                    .into(),
            );
        }
        self.identities
            .insert(definition.path.clone(), definition.clone());
        Ok(())
    }
}

struct Nominals<'a> {
    input: &'a Nominal,
    span: &'a Nominal,
    anchored: &'a Nominal,
    input_type: &'a TypeTree,
    span_type: &'a TypeTree,
    anchored_type: &'a TypeTree,
}

impl<'a> Nominals<'a> {
    fn new(interface: &'a InterfaceObservations) -> Result<Self> {
        if interface.identity != crate::concrete_types::IDENTITY || interface.parameters.len() != 1
        {
            return Err(
                "Constructor requires complete original concrete interface observations".into(),
            );
        }
        let Captured::Captured { ty: input_type } = &interface.result else {
            return Err("Constructor lacks its complete canonical Input result shape".into());
        };
        let TypeTree::Struct {
            nominal: input,
            fields,
            ..
        } = input_type
        else {
            return Err("Constructor canonical result is not the original Input struct".into());
        };

        field_names(fields, &["haystack", "span", "anchored", "earliest"])?;
        let expected = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
            "u8".into(),
        )))));
        if fields[0].ty != expected || fields[3].ty != TypeTree::Scalar("bool".into()) {
            return Err(
                "Constructor canonical view/earliest components differ from the source contract"
                    .into(),
            );
        }
        let (span, anchored) = components(input, fields)?;
        Ok(Self {
            input,
            span,
            anchored,
            input_type,
            span_type: &fields[1].ty,
            anchored_type: &fields[2].ty,
        })
    }

    fn get(&self, role: Role) -> Result<(&'a Nominal, usize)> {
        match role {
            Role::Input => Ok((self.input, 1)),
            Role::Span => Ok((self.span, 0)),
            Role::Anchored => Ok((self.anchored, 0)),
            _ => Err("Unknown constructor nominal component role".into()),
        }
    }
}

fn field_names(fields: &[crate::concrete_types::Field], names: &[&str]) -> Result<()> {
    if !fields
        .iter()
        .map(|field| (field.position, field.name.as_str()))
        .eq(names.iter().copied().enumerate())
    {
        return Err(
            "Constructor canonical component inventory omits/reorders/duplicates fields".into(),
        );
    }
    Ok(())
}

impl ViewType for Type {
    const HAYSTACK_ERROR: &'static str = "Constructor projection requires concrete str/[u8]";

    fn u8() -> Self {
        Self::Builtin { name: "u8".into() }
    }

    fn text() -> Self {
        Self::Str
    }

    fn slice(element: Self) -> Self {
        Self::Slice {
            element: Box::new(element),
        }
    }
}

impl BorrowedViewType for Type {
    fn reference(self, region: String, mutable: bool) -> Self {
        Self::Reference {
            region,
            mutable,
            referent: Box::new(self),
        }
    }
}

pub(super) fn shared_bytes_token(text: &str) -> Result<bool> {
    let token: crate::data_types::TypeToken =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    Ok(token == normalized::shared_bytes())
}

fn components<'a>(
    input: &Nominal,
    fields: &'a [crate::concrete_types::Field],
) -> Result<(&'a Nominal, &'a Nominal)> {
    let TypeTree::Struct {
        nominal: span,
        fields: endpoints,
        ..
    } = &fields[1].ty
    else {
        return Err("Constructor canonical span component is not a complete struct".into());
    };
    field_names(endpoints, &["start", "end"])?;
    if endpoints
        .iter()
        .any(|field| field.ty != TypeTree::Scalar("usize".into()))
    {
        return Err("Constructor span endpoints do not have concrete usize identity".into());
    }

    let (anchored, _) = fields[2]
        .ty
        .nominal_parts()
        .ok_or("Constructor anchored component lacks canonical nominal source")?;
    if input.definition_path != "util::search::Input"
        || span.definition_path != "util::search::Span"
        || anchored.definition_path != "util::search::Anchored"
    {
        return Err(
            "Constructor canonical result changes one of the original nominal components".into(),
        );
    }
    Ok((span, anchored))
}

pub(super) fn paths(canonical: &HirFnFacts, paths: &[(Role, syn::TypePath)]) -> Result<()> {
    let interface = canonical
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let nominals = Nominals::new(&interface)?;
    if paths.len() != 3 {
        return Err("Constructor complete nominal path roster differs".into());
    }
    let expected = [Role::Input, Role::Span, Role::Anchored];

    let checked = paths
        .iter()
        .zip(expected)
        .map(|((role, path), expected)| {
            if *role != expected {
                return Err("Constructor lexical path roster changes component order".into());
            }
            let (nominal, _) = nominals.get(*role)?;
            Ok((path.clone(), nominal.clone()))
        })
        .collect::<Result<Vec<_>>>()?;
    crate::nominal_boundary::resolution::verify_reference_paths(canonical, &checked)
}
