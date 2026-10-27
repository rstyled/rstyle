//! Exact reader and validator propagation with independently no-drop named errors.
mod authority;
pub mod capture;
#[cfg(test)]
mod tests;

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use capture::Witness;

pub const CAPABILITY: &str = "rust.named-error-reader-flow";

pub const PRODUCT_CAPABILITY: &str = "rust.named-error-product-reader-flow";

pub const VALIDATOR_CAPABILITY: &str = "rust.named-error-validator-flow";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    UnitToReader,
    UnitToU16Reader,
    ReaderToScalar,
    ReaderToCopyProduct,
    IdToReader,
}

impl Mode {
    pub const fn capability(self) -> &'static str {
        match self {
            Self::UnitToReader => CAPABILITY,
            Self::UnitToU16Reader => crate::native_reader_width::CAPABILITY,
            Self::ReaderToScalar => VALIDATOR_CAPABILITY,
            Self::ReaderToCopyProduct => PRODUCT_CAPABILITY,
            Self::IdToReader => crate::id_reader_types::CAPABILITY,
        }
    }

    pub const fn identity(self) -> &'static str {
        match self {
            Self::UnitToReader => capture::IDENTITY,
            Self::UnitToU16Reader => capture::U16_IDENTITY,
            Self::ReaderToScalar => capture::VALIDATOR_IDENTITY,
            Self::ReaderToCopyProduct => capture::PRODUCT_IDENTITY,
            Self::IdToReader => capture::ID_READER_IDENTITY,
        }
    }

    pub const fn resolution(self) -> &'static str {
        match self {
            Self::UnitToReader => capture::RESOLUTION,
            Self::UnitToU16Reader => capture::U16_RESOLUTION,
            Self::ReaderToScalar => capture::VALIDATOR_RESOLUTION,
            Self::ReaderToCopyProduct => capture::PRODUCT_RESOLUTION,
            Self::IdToReader => capture::ID_READER_RESOLUTION,
        }
    }

    pub const fn constructs(self) -> [&'static str; 3] {
        match self {
            Self::UnitToU16Reader => [
                "try.u16-reader-result-is-err",
                "try.u16-reader-result-error",
                "try.u16-reader-result-ok-unit",
            ],
            Self::UnitToReader => [
                "try.reader-result-is-err",
                "try.reader-result-error",
                "try.reader-result-ok-unit",
            ],
            Self::ReaderToCopyProduct => [
                "try.product-reader-result-is-err",
                "try.product-reader-result-error",
                "try.product-reader-result-ok-tuple",
            ],
            Self::IdToReader => [
                "try.id-reader-result-is-err",
                "try.id-reader-result-error",
                "try.id-reader-result-ok-id",
            ],
            Self::ReaderToScalar => [
                "try.validator-result-is-err",
                "try.validator-result-error",
                "try.validator-result-ok-tuple",
            ],
        }
    }

    pub(super) fn from_identity(identity: &str) -> Result<Self, String> {
        match identity {
            capture::IDENTITY => Ok(Self::UnitToReader),
            capture::U16_IDENTITY => Ok(Self::UnitToU16Reader),
            capture::VALIDATOR_IDENTITY => Ok(Self::ReaderToScalar),
            capture::PRODUCT_IDENTITY => Ok(Self::ReaderToCopyProduct),
            capture::ID_READER_IDENTITY => Ok(Self::IdToReader),
            _ => Err("Unregistered named-error propagation identity".into()),
        }
    }
}

pub struct Checked<'a> {
    pub mode: Mode,
    pub operand: &'a HirNode,
    pub early_return: &'a HirNode,
    pub input_token: String,
    pub output_token: String,
    pub payload_token: String,
    pub error_token: String,
}

pub fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    mode(node, facts).is_some()
}

pub fn mode_candidate(node: &HirNode, facts: &dyn EvidenceFacts, expected: Mode) -> bool {
    mode(node, facts) == Some(expected)
}

fn mode(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Mode> {
    let resolution = node.res.as_ref()?;
    if node.kind != "expr_try_desugar" {
        return None;
    }
    [
        Mode::UnitToReader,
        Mode::UnitToU16Reader,
        Mode::ReaderToScalar,
        Mode::ReaderToCopyProduct,
        Mode::IdToReader,
    ]
    .into_iter()
    .find(|mode| {
        resolution.kind == mode.resolution()
            && crate::extraction_profile::facts_support(facts, mode.capability())
    })
}

pub fn verify<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Checked<'a>, String> {
    let (mode, witness) = checked_witness(node, facts)?;

    authority::concrete_shape_for(&witness.shape, mode)?;
    let (operand, early_return, residual) =
        crate::scalar_result_propagation::structure_with_generated(
            node,
            &witness,
            &witness.shape.output,
        )?;

    let input_token = crate::data_aggregates::token(operand)?.to_owned();

    let output_token = facts
        .signature
        .return_ty
        .normalized
        .as_ref()
        .ok_or("Missing complete Result return type")?
        .clone();

    let payload_token = crate::data_aggregates::token(node)?.to_owned();

    let error_token = types(node, operand, facts, mode)?;

    if crate::data_aggregates::token(residual)? != output_token
        || crate::data_aggregates::token(early_return)? != output_token
    {
        return Err("Residual conversion must construct the complete output Result".into());
    }

    authority::verify(&witness, operand, facts)?;

    crate::result_propagation::source::verify(node, operand, facts)?;

    Ok(Checked {
        mode,
        operand,
        early_return,
        input_token,
        output_token,
        payload_token,
        error_token,
    })
}

fn checked_witness(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(Mode, Witness), String> {
    if !candidate(node, facts) || node.expansion.is_some() || node.dispatch.is_some() {
        return Err(
            "Named error reader Result propagation requires its original supported question mark"
                .into(),
        );
    }

    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing named error reader witness")?;

    if resolution.target.is_some() {
        return Err("Named error reader Result witness has an unrelated target".into());
    }

    let witness: Witness =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;

    let mode = mode(node, facts).ok_or("Named-error propagation lacks its active mode")?;
    if witness.identity != mode.identity()
        || witness.root != node.hir
        || witness.try_span != node.span
    {
        return Err("Named error reader Result witness identifies another expression".into());
    }

    Ok((mode, witness))
}

fn types(
    node: &HirNode,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
    mode: Mode,
) -> Result<String, String> {
    let input = crate::data_aggregates::parsed_type(operand)?;

    let output = crate::data_types::parse(
        facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .ok_or("Missing complete Result return type")?,
    )?;

    let payload = crate::data_aggregates::parsed_type(node)?;

    let TypeToken::Result { ok, err } = input else {
        return Err("Named error reader propagation operand is not Result".into());
    };

    let TypeToken::Result {
        ok: output_ok,
        err: output_err,
    } = output
    else {
        return Err("Named error reader propagation return is not Result".into());
    };

    if *ok != payload || !payloads_match(mode, &ok, &output_ok) || err != output_err {
        return Err(
            "Named-error propagation changes its exact payload transition or identical error"
                .into(),
        );
    }
    if matches!(mode, Mode::ReaderToCopyProduct | Mode::IdToReader) {
        copy_product_boundary(&output_ok, &output_err, facts, mode)?;
    }

    let error = crate::named_error_payloads::error_tree(facts)?
        .ok_or("Reader propagation lacks independently checked named error ownership")?;
    if TypeToken::from_tree(&error) != *err {
        return Err("Reader propagation error differs from its complete no-drop boundary".into());
    }
    err.text()
}

fn copy_product_boundary(
    success: &TypeToken,
    error: &TypeToken,
    facts: &dyn EvidenceFacts,
    mode: Mode,
) -> Result<(), String> {
    use crate::concrete_types::{InterfaceObservations, Observation};
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    if !matches!(&interface.result, Observation::Captured { ty }
    if TypeToken::from_tree(ty) == TypeToken::Result {
        ok: Box::new(success.clone()), err: Box::new(error.clone())
    }) || !match mode {
        Mode::IdToReader => crate::id_reader_types::success(success, facts)?,
        _ => crate::reader_products::success(success, facts)?,
    } {
        return Err(
            "Product reader output lacks complete independently checked Copy ownership".into(),
        );
    }
    Ok(())
}

fn payloads_match(mode: Mode, input: &TypeToken, output: &TypeToken) -> bool {
    match mode {
        Mode::UnitToU16Reader => {
            *input == TypeToken::Scalar("unit".into())
                && crate::native_reader_width::Width::for_success(output)
                    == Some(crate::native_reader_width::Width::U16)
        }
        Mode::UnitToReader => *input == TypeToken::Scalar("unit".into()) && reader_tuple(output),
        Mode::ReaderToScalar => reader_tuple(input) && *output == TypeToken::Scalar("usize".into()),
        Mode::ReaderToCopyProduct => reader_tuple(input) && crate::reader_products::shape(output),
        Mode::IdToReader => {
            crate::id_reader_types::shape(output)
                && matches!(output, TypeToken::Tuple(values) if values.first() == Some(input))
        }
    }
}

pub(crate) fn success(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Scalar(value) if value == "unit") || reader_tuple(token)
}

fn reader_tuple(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Tuple(values) if matches!(values.as_slice(),
        [TypeToken::Scalar(first), TypeToken::Scalar(second)]
        if first == "u32" && second == "usize"))
}
