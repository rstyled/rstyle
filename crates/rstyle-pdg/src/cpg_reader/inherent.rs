//! A local inherent method's own definition record: the implementation its calls are
//! verified through, and the nominal owner the implementation is for.

use super::{Context, located};
use crate::data_types::TypeToken;
use crate::frontend::{ByteSpan, HirFnFacts};
use crate::inherent_calls::{Definition, IDENTITY, KEY};

impl Context<'_, '_> {
    pub(super) fn inherent_definition(&self, facts: &mut HirFnFacts) -> Result<(), String> {
        let function = self.body.function;
        let Ok(definition) = self.reader.tables.definition(&function.definition) else {
            return Ok(());
        };
        let Some(implementation) = self.inherent_implementation(definition) else {
            return Ok(());
        };
        // An expanded function keeps its facts, but no unique original definition.
        let expanded = !std::ptr::eq(located(function), &function.span);
        let profile = self.reader.profile;
        let has_self = facts.signature.has_self;
        if expanded || !has_self && !profile.supports("rust.static-inherent-helper-calls") {
            return Ok(());
        }
        let nominal = self.owner(definition, facts)?;
        let Some(owner_nominal) = nominal else {
            return Ok(());
        };

        let span = implementation
            .span
            .as_ref()
            .ok_or("Missing original inherent implementation span")?;
        let identity = |definition, key| {
            super::identity(definition, key)
                .map(str::to_owned)
                .ok_or("Missing inherent compiler identity")
        };
        let record = Definition {
            identity: IDENTITY.into(),
            definition_identity: identity(definition, "def_path_hash")?,
            implementation_identity: identity(implementation, "def_path_hash")?,
            implementation_span: ByteSpan {
                start: span.start as u32,
                end: span.end as u32,
            },
            owner_nominal,
            source_file: facts
                .build
                .frontend_options
                .get("source_file")
                .and_then(serde_json::Value::as_str)
                .ok_or("Missing inherent source file")?
                .into(),
            function_span: facts.fn_span,
        };
        let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
        facts.build.frontend_options.insert(KEY.into(), value);
        Ok(())
    }
}

impl Context<'_, '_> {
    /// The nominal owner of a method: its receiver's, or a static method's
    /// implementation's.
    fn owner(
        &self,
        definition: &rstyle_core::cpg::model::Definition,
        facts: &HirFnFacts,
    ) -> Result<Option<crate::concrete_types::Nominal>, String> {
        if facts.signature.has_self {
            return Ok(owner_nominal(facts, self.reader.profile));
        }
        match definition
            .callable
            .as_ref()
            .and_then(|callable| callable.owner_type.as_deref())
        {
            Some(owner) => self.static_owner(owner),
            None => Ok(None),
        }
    }
}

/// The nominal type a method's receiver is, or borrows, where the profile verifies it.
fn owner_nominal(
    facts: &HirFnFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Option<crate::concrete_types::Nominal> {
    let token = facts.signature.params.first()?.ty.normalized.as_deref()?;
    let owner = match crate::data_types::parse(token).ok()? {
        TypeToken::Shared(owner) => *owner,
        owner @ TypeToken::Struct { .. } if profile.supports("rust.owned-copy-self") => owner,
        owner @ TypeToken::FieldlessEnum { .. }
            if profile.supports("rust.fieldless-enum-values") =>
        {
            owner
        }
        _ => return None,
    };
    match owner {
        TypeToken::Struct { nominal, .. } | TypeToken::FieldlessEnum { nominal, .. } => {
            Some(nominal)
        }
        _ => None,
    }
}
