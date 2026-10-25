//! Typed concrete observations at one selected site, not a generic instance model.
use crate::HirFnFacts;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

impl Definition {
    pub fn local(&self, facts: &HirFnFacts) -> Result<(), String> {
        let compiled = facts
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            .ok_or("Local value trait lacks its original compiled-crate identity")?;
        if !self.local
            || self.path.is_empty()
            || self.definition_identity.is_empty()
            || self.stable_crate_id.is_empty()
            || compiled["identity"] != "rstyle.rust.compiled-crate/1"
            || compiled["crate_name"].as_str() != Some(self.crate_name.as_str())
            || compiled["stable_crate_id"].as_str() != Some(self.stable_crate_id.as_str())
        {
            return Err("Local value trait definition differs from its compiled owner".into());
        }
        Ok(())
    }

    pub fn selected(&self, facts: &HirFnFacts) -> Result<(), String> {
        self.local(facts)?;
        crate::source_evidence::require_definition(facts, &self.path, &self.definition_identity)
    }
}

pub(super) fn original(facts: &HirFnFacts) -> Result<Definition, String> {
    let options = &facts.build.frontend_options;
    let compiled = options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Local value trait original compiled owner missing")?;
    let definition = Definition {
        path: facts.name.clone(),
        definition_identity: options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            .ok_or("Local value trait original method identity missing")?
            .into(),
        crate_name: compiled["crate_name"]
            .as_str()
            .ok_or("Original local crate name missing")?
            .into(),
        stable_crate_id: compiled["stable_crate_id"]
            .as_str()
            .ok_or("Original local stable crate identity missing")?
            .into(),
        local: true,
    };

    definition.local(facts)?;
    Ok(definition)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    kind: String,
    value: String,
    display_only: bool,
    identity_status: String,
    identity: Builtin,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Builtin {
    Builtin { name: String },
}

pub(super) fn self_argument(arguments: &[Argument], input: &str) -> Result<(), String> {
    let [argument] = arguments else {
        return Err("Local value trait requires its single concrete Self substitution".into());
    };
    let Builtin::Builtin { name } = &argument.identity;
    if argument.kind != "type"
        || argument.value.is_empty()
        || !argument.display_only
        || argument.identity_status != "known"
        || name != input
    {
        return Err("Local value trait Self lacks its structural builtin input identity".into());
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Call {
    pub hir: crate::HirRef,
    pub span: crate::ByteSpan,
    pub expanded: bool,
    pub status: String,
    pub requested: Definition,
    pub selected: Definition,
    pub requested_arguments: Vec<Argument>,
    pub selected_arguments: Vec<Argument>,
    pub signature: serde_json::Value,
    #[serde(default)]
    pub compiled_dependency: Option<serde_json::Value>,
}
