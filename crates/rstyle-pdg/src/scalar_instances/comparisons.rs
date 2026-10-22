//! Exact pinned primitive trait comparisons; overload observations stay intact.
//!
//! The owning instance/source bridge must separately verify substitution and
//! whole-body correspondence. This leaf authorizes only the recorded operation.
use crate::HirNode;
use serde::Deserialize;

mod pins;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scalar {
    I32,
    U32,
}

impl Scalar {
    fn name(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::U32 => "u32",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Operator {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "==" => Ok(Self::Eq),
            "!=" => Ok(Self::Ne),
            "<" => Ok(Self::Lt),
            "<=" => Ok(Self::Le),
            ">" => Ok(Self::Gt),
            ">=" => Ok(Self::Ge),
            _ => Err("Unregistered primitive comparison operator".into()),
        }
    }

    fn method(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Gt => "gt",
            Self::Ge => "ge",
        }
    }

    fn trait_name(self) -> &'static str {
        match self {
            Self::Eq | Self::Ne => "PartialEq",
            _ => "PartialOrd",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Checked {
    pub scalar: Scalar,
    pub operator: Operator,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    identity: String,
    hir: crate::HirRef,
    span: crate::ByteSpan,
    expanded: bool,
    operator: String,
    requested: Definition,
    requested_trait: Definition,
    original_arguments: Vec<Argument>,
    status: String,
    requested_arguments: Vec<Argument>,
    selected: Definition,
    selected_arguments: Vec<Argument>,
    selected_impl: Definition,
    selected_trait: Definition,
    requested_signature: Signature,
    signature: Signature,
    operand_types: Vec<Argument>,
    result_type: Vec<Argument>,
    operand_adjustments: [Adjustments; 2],
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    path: String,
    definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
    crate_hash: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display_value: String,
    display_only: bool,
    identity_status: String,
    identity: Option<Type>,
    reason: Option<String>,
}

// Compiler display strings remain diagnostic, never chain identity.
impl PartialEq for Argument {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.display_only == other.display_only
            && self.identity_status == other.identity_status
            && self.identity == other.identity
            && self.reason == other.reason
    }
}

impl Eq for Argument {}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Type {
    Builtin {
        name: String,
    },
    Reference {
        region: String,
        mutable: bool,
        referent: Box<Type>,
    },
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    inputs: Vec<Argument>,
    output: Vec<Argument>,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Adjustments {
    pub(super) hir: crate::HirRef,
    pub(super) span: crate::ByteSpan,
    source_type: Vec<Argument>,
    steps: Vec<Step>,
    adjusted_type: Vec<Argument>,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Step {
    kind: String,
    target: Vec<Argument>,
}

pub(super) fn site(record: &Observation) -> &crate::HirRef {
    &record.hir
}

pub(super) fn adjustment_site(record: &Adjustments) -> &crate::HirRef {
    &record.hir
}

pub(super) fn operand_sites(record: &Observation) -> [&crate::HirRef; 2] {
    [
        &record.operand_adjustments[0].hir,
        &record.operand_adjustments[1].hir,
    ]
}

/// The parent verifies the complete census and original return/control-flow
/// witness. This checks only the exact recorded adjustment and node endpoints.
pub(super) fn verify_adjustments(
    node: &HirNode,
    record: &Adjustments,
    comparison: Option<&Observation>,
    scalar: &str,
) -> Result<(), String> {
    if !matches!(scalar, "i32" | "u32")
        || record.hir != node.hir
        || record.span != node.span
        || record.steps.is_empty()
    {
        return Err("Scalar instance adjustment loses its exact nonempty source site".into());
    }
    let scalar_type = Type::Builtin {
        name: scalar.into(),
    };
    let borrowed = Type::Reference {
        region: "erased".into(),
        mutable: false,
        referent: Box::new(scalar_type.clone()),
    };
    let comparison_operand = comparison
        .filter(|record| record.status == "resolved")
        .is_some_and(|comparison| {
            comparison
                .operand_adjustments
                .iter()
                .any(|operand| operand == record)
        });
    if comparison_operand {
        return adjustment(node, record, &scalar_type, &borrowed);
    }

    never_adjustment(node, record, &scalar_type, scalar)
}

fn never_adjustment(
    node: &HirNode,
    record: &Adjustments,
    scalar: &Type,
    scalar_name: &str,
) -> Result<(), String> {
    let [step] = record.steps.as_slice() else {
        return Err("Return continuation requires one exact NeverToAny adjustment".into());
    };
    if node.kind != "expr_ret"
        || node.children.len() != 1
        || step.kind != "never_to_any"
        || crate::frontend::normalized(node) != Some(scalar_name)
        || crate::frontend::normalized(&node.children[0]) != Some(scalar_name)
    {
        return Err(
            "Scalar adjustment is neither a verified comparison autoref nor a return continuation"
                .into(),
        );
    }

    list(&record.source_type, &[&Type::Builtin { name: "!".into() }])?;
    list(&step.target, &[scalar])?;
    list(&record.adjusted_type, &[scalar])
}

pub(super) fn verify(
    node: &HirNode,
    record: &Observation,
    source: &super::source::Checked<'_>,
) -> Result<Checked, String> {
    // The private orchestrator indexes nodes from this exact checked body.
    let facts = source.facts();
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        != crate::extraction_profile::ExtractionProfile::DataV3
    {
        return Err("Primitive comparison instances require the DataV3 profile".into());
    }

    let checked = values(node, record)?;
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Primitive comparison requires pinned compiler/target authority")?;
    pins::verify(record, checked, registry)?;
    Ok(checked)
}

fn values(node: &HirNode, record: &Observation) -> Result<Checked, String> {
    let operator = Operator::parse(&record.operator)?;
    let [left, right] = node.children.as_slice() else {
        return Err("Primitive comparison requires two complete operands".into());
    };
    let scalar = match crate::frontend::normalized(left) {
        Some("i32") => Scalar::I32,
        Some("u32") => Scalar::U32,
        _ => return Err("Primitive comparison requires exact i32/u32 operands".into()),
    };

    let checked = Checked { scalar, operator };
    let scalar_type = Type::Builtin {
        name: scalar.name().into(),
    };
    let borrowed = Type::Reference {
        region: "erased".into(),
        mutable: false,
        referent: Box::new(scalar_type.clone()),
    };

    shape(node, record, checked)?;
    list(&record.requested_arguments, &[&scalar_type, &scalar_type])?;
    list(&record.operand_types, &[&scalar_type, &scalar_type])?;
    list(
        &record.result_type,
        &[&Type::Builtin {
            name: "bool".into(),
        }],
    )?;

    signature(&record.requested_signature, &borrowed)?;
    signature(&record.signature, &borrowed)?;
    for (operand, adjustments) in [left, right].into_iter().zip(&record.operand_adjustments) {
        adjustment(operand, adjustments, &scalar_type, &borrowed)?;
    }
    Ok(checked)
}

fn shape(node: &HirNode, record: &Observation, checked: Checked) -> Result<(), String> {
    let name = checked.scalar.name();
    let valid = record.identity == "rstyle.rust.primitive-comparison-observation/1"
        && record.status == "resolved"
        && record.hir == node.hir
        && record.span == node.span
        && record.expanded == node.expansion.is_some()
        && !record.expanded
        && record.original_arguments.len() == 2
        && record.selected_arguments.is_empty()
        && node.kind == "expr_binary"
        && !node.is_unsafe_block
        && node
            .children
            .iter()
            .all(|child| crate::frontend::normalized(child) == Some(name))
        && crate::frontend::normalized(node) == Some("bool")
        && node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "overloaded"
                && dispatch.op == record.operator
                && dispatch.operand_tys == [name, name]
                && dispatch.result_ty == "bool"
        });
    if !valid {
        return Err("Primitive comparison loses original overloaded node/type identity".into());
    }
    Ok(())
}

fn signature(signature: &Signature, borrowed: &Type) -> Result<(), String> {
    list(&signature.inputs, &[borrowed, borrowed])?;
    list(
        &signature.output,
        &[&Type::Builtin {
            name: "bool".into(),
        }],
    )
}

fn list(arguments: &[Argument], expected: &[&Type]) -> Result<(), String> {
    if arguments.len() != expected.len()
        || !arguments.iter().zip(expected).all(|(argument, ty)| {
            argument.kind == "type"
                && argument.display_only
                && argument.identity_status == "known"
                && argument.reason.is_none()
                && argument.identity.as_ref() == Some(*ty)
        })
    {
        return Err("Primitive comparison lacks exact structural type identity".into());
    }
    Ok(())
}

fn adjustment(
    node: &HirNode,
    record: &Adjustments,
    scalar: &Type,
    borrowed: &Type,
) -> Result<(), String> {
    let [step] = record.steps.as_slice() else {
        return Err("Primitive operand requires exactly one compiler shared autoref".into());
    };
    if record.hir != node.hir || record.span != node.span || step.kind != "shared_borrow" {
        return Err("Primitive operand adjustment loses its exact source/autoref identity".into());
    }

    list(&record.source_type, &[scalar])?;
    list(&step.target, &[borrowed])?;
    list(&record.adjusted_type, &[borrowed])
}
