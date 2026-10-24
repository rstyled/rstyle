//! Compiler-evaluated local bool tables; declarations never execute at runtime.
mod declarations;

use crate::{HirNode, PathRes, unit_store::EvidenceFacts};
pub use declarations::{array_declaration, function_declaration};
use serde::{Deserialize, Serialize};

pub const CAPABILITY: &str = "rust.local-const-bool-tables";
pub const RESOLUTION: &str = "resolved_bool_array_constant";
pub const FUNCTION: &str = "resolved_const_array_function";
pub const MAX_LENGTH: u64 = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoolArray {
    pub definition: String,
    pub declaration: String,
    pub ty: String,
    pub values: Vec<bool>,
    pub initializer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstFunction {
    pub definition: String,
    pub declaration: String,
    pub ty: String,
}

pub fn candidate(node: &HirNode) -> bool {
    matches!(
        node.kind.as_str(),
        "stmt_bool_array" | "stmt_const_array_fn"
    ) || node.res.as_ref().is_some_and(|res| res.kind == RESOLUTION)
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.kind == "expr_path" {
        verify_use(node, facts).map(|_| ())
    } else {
        verify_declaration(node, facts)
    }
}

impl BoolArray {
    pub fn decode(resolution: &PathRes) -> Result<Self, String> {
        crate::scalar_constants::decode_record(resolution, RESOLUTION)
    }

    pub fn literal(&self) -> String {
        serde_json::to_string(&self.values).expect("bool array serializes")
    }
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    for node in &facts.body.children {
        if matches!(
            node.kind.as_str(),
            "stmt_bool_array" | "stmt_const_array_fn"
        ) {
            verify_declaration(node, facts)?;
        }
    }
    Ok(())
}

pub(crate) fn verify_declaration(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || node.expansion.is_some()
        || !node.children.is_empty()
        || !facts
            .body
            .children
            .iter()
            .any(|child| child.hir == node.hir)
    {
        return Err("Const tables require original top-level local declarations".into());
    }
    let source = source(node, facts)?;
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing compile-time declaration resolution")?;
    if node.kind == "stmt_const_array_fn" {
        let record: ConstFunction = crate::scalar_constants::decode_record(resolution, FUNCTION)?;
        let item = function_declaration(source)?;
        check_owner(&record.definition, &item.sig.ident.to_string(), facts)?;
        if record.declaration != source || record.ty != declarations::return_token(&item.sig)? {
            return Err("Compile-time function loses complete source or return type".into());
        }
        return Ok(());
    }
    if node.kind != "stmt_bool_array" {
        return Err("Expected a compile-time table declaration".into());
    }

    let record = BoolArray::decode(resolution)?;
    let item = array_declaration(source)?;
    check_owner(&record.definition, &item.ident.to_string(), facts)?;
    if record.declaration != source
        || record.ty != declarations::array_token(&item.ty)?
        || record.values.len() as u64 != declarations::array_length(&item.ty)?
    {
        return Err("Evaluated table loses its exact declaration, type or complete value".into());
    }
    verify_initializer(&record, &item, facts)
}

fn verify_initializer(
    record: &BoolArray,
    item: &syn::ItemConst,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let name = declarations::initializer(item)?;
    let expected = record
        .definition
        .rsplit_once("::")
        .ok_or("Missing table owner")?
        .0;
    if record.initializer != format!("{expected}::{name}") {
        return Err("Const table initializer loses its exact local function identity".into());
    }

    let mut matching = facts.body.children.iter().filter(|node| {
        node.kind == "stmt_const_array_fn"
            && node.res.as_ref().is_some_and(|res| {
                crate::scalar_constants::decode_record::<ConstFunction>(res, FUNCTION).is_ok_and(
                    |function| {
                        function.definition == record.initializer && function.ty == record.ty
                    },
                )
            })
    });
    let function = matching
        .next()
        .ok_or("Missing complete local table initializer declaration")?;
    if matching.next().is_some() {
        return Err("Ambiguous table initializer declaration".into());
    }
    verify_declaration(function, facts)
}

pub(crate) fn verify_use(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<BoolArray, String> {
    if node.kind != "expr_path" || node.expansion.is_some() || !node.children.is_empty() {
        return Err("Const table requires an original value path".into());
    }
    let record = BoolArray::decode(node.res.as_ref().ok_or("Missing table resolution")?)?;
    let path: syn::ExprPath = syn::parse_str(source(node, facts)?).map_err(|e| e.to_string())?;
    let name = record
        .definition
        .rsplit("::")
        .next()
        .ok_or("Missing table name")?;
    if path.qself.is_some()
        || !path.attrs.is_empty()
        || !path.path.is_ident(name)
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(&record.ty)
    {
        return Err("Const table use loses its exact lexical name/type".into());
    }

    let declarations: Vec<_> = facts
        .body
        .children
        .iter()
        .filter(|node| {
            node.kind == "stmt_bool_array"
                && node.res.as_ref().is_some_and(|res| {
                    BoolArray::decode(res).is_ok_and(|declaration| declaration == record)
                })
        })
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err("Const table use lacks one identical owned declaration".into());
    };
    verify_declaration(declaration, facts)?;
    Ok(record)
}

fn check_owner(definition: &str, name: &str, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if definition != format!("{}::{name}", facts.name) {
        return Err("Const declaration belongs to a different function".into());
    }
    Ok(())
}

fn source<'a>(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Result<&'a str, String> {
    facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or_else(|| "Missing complete compile-time declaration source".into())
}
