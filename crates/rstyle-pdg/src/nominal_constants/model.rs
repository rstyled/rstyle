//! Independent observed values, selected definitions, and original source coordinates.
use crate::{ByteSpan, concrete_types::TypeTree, data_library::Callee};
use serde::{Deserialize, Serialize};

pub const IDENTITY: &str = "rstyle.rust.product-constant/1";
pub const RESOLUTION: &str = "resolved_product_constant";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSite {
    pub file: String,
    pub span: ByteSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarValue {
    pub ty: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreConstant {
    pub definition: Callee,
    pub scalar: ScalarValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Initializer {
    pub span: ByteSpan,
    pub core_constant: CoreConstant,
    pub offset: String,
    pub cast_from: String,
    pub cast_to: String,
    pub subtraction_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constructor {
    pub definition: Callee,
    pub implementation: Callee,
    pub declaration: SourceSite,
    pub implementation_source: SourceSite,
    pub parameter_span: ByteSpan,
    pub tuple_constructor: Callee,
    pub parameter_type: String,
    pub result_type: String,
    pub field_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductConstant {
    pub identity: String,
    pub definition: Callee,
    pub implementation: Callee,
    pub declaration: SourceSite,
    pub implementation_source: SourceSite,
    pub ty: TypeTree,
    pub field: ScalarValue,
    pub initializer: Initializer,
    pub constructor: Constructor,
}

pub(super) fn shape(record: &ProductConstant) -> Result<(), String> {
    if record.identity != IDENTITY {
        return Err("Unregistered product constant observation identity".into());
    }
    let TypeTree::Struct { fields, .. } = &record.ty else {
        return Err("Product constant requires a complete nominal struct".into());
    };
    if fields.len() != 1
        || fields[0].position != 0
        || fields[0].name != "0"
        || fields[0].ty != TypeTree::Scalar("u32".into())
        || record.field.ty != "u32"
        || record.constructor.parameter_type != "usize"
        || record.constructor.field_type != "u32"
        || record.constructor.result_type != crate::data_types::token(&record.ty)?
    {
        return Err("Product constant requires one exact u32 tuple field".into());
    }

    let definitions = [
        &record.definition,
        &record.implementation,
        &record.constructor.definition,
        &record.constructor.implementation,
        &record.constructor.tuple_constructor,
    ];
    for definition in definitions {
        if definition.crate_name != record.definition.crate_name
            || definition.crate_hash != "local-unencoded"
            || definition.crate_name.is_empty()
            || definition.path.is_empty()
            || definition.definition_hash.is_empty()
            || definition.implementation.is_some()
        {
            return Err("Product constant lacks complete same-crate local identities".into());
        }
    }
    Ok(())
}
