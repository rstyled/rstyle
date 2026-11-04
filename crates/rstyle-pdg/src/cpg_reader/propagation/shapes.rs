//! The type shapes each propagation protocol accepts, as the frontend prints them.

use super::{CapturedShape, Protocol};
use crate::cpg_reader::{Context, identity};
use crate::named_error_result_propagation::Mode;
use crate::result_propagation::capture::Selection;
use rstyle_core::cpg::model::{Node, Scope, Type, TypeKind};

type Shaped = Option<(CapturedShape, Option<Selection>)>;

/// The types a `Result` propagation relates, each a type ID.
struct Flow<'a> {
    input: &'a str,
    payload: &'a str,
    error: &'a str,
    output: &'a str,
    output_payload: &'a str,
    residual: &'a str,
    control_flow: &'a str,
}

impl<'a> Context<'_, 'a> {
    pub(super) fn protocol_shape(
        &self,
        branch: &'a Node,
        operand: &'a Node,
        protocol: Protocol,
    ) -> Shaped {
        let input = operand.ty.as_deref()?;
        let control_flow = branch.ty.as_deref()?;
        if let Protocol::Option = protocol {
            return Some((
                CapturedShape::Option(self.option_shape(input, control_flow)?),
                None,
            ));
        }
        let (payload, error) = self.payloads(input)?;
        let shape = match protocol {
            Protocol::Result => CapturedShape::Result(self.same_error_shape(input, control_flow)?),
            _ => {
                let output = self.body.function.signature.result.as_str();
                let (output_payload, output_error) = self.payloads(output)?;
                if error != output_error {
                    return None;
                }
                let residual = self.checked_result_residual(control_flow, payload, error)?;
                let flow = Flow {
                    input,
                    payload,
                    error,
                    output,
                    output_payload,
                    residual,
                    control_flow,
                };
                self.flow_shape(protocol, &flow)?
            }
        };

        Some((shape, Some(self.propagation_conversion(error)?)))
    }

    fn flow_shape(&self, protocol: Protocol, flow: &Flow<'_>) -> Option<CapturedShape> {
        let text = |id: &str| self.spelling(id);
        let accepted = match protocol {
            Protocol::NamedErrorResult(mode) => {
                self.named_error_payloads(mode, flow.payload, flow.output_payload)
                    && self.local_plain_struct(flow.error)
            }
            Protocol::BorrowedResult => {
                self.byte_slice(flow.payload)
                    && matches!(self.items(flow.output_payload).as_deref(), Some([size, rest])
                        if self.reader.integer(Some(size), "usize") && self.byte_slice(rest))
            }
            Protocol::ScalarResult => {
                self.reader.integer(Some(flow.payload), "usize")
                    && self.byte_slice(flow.output_payload)
            }
            Protocol::ProductResult => {
                self.reader
                    .ty(flow.payload)
                    .is_ok_and(|ty| ty.kind == TypeKind::Record)
                    && erased(&text(flow.input)?) == erased(&text(flow.output)?)
            }
            Protocol::Result | Protocol::Option => false,
        };
        if !accepted {
            return None;
        }
        macro_rules! shape {
            ($variant:ident, $module:ident) => {
                CapturedShape::$variant(crate::$module::capture::Shape {
                    input: text(flow.input)?,
                    output: text(flow.output)?,
                    payload: text(flow.payload)?,
                    output_payload: text(flow.output_payload)?,
                    error: text(flow.error)?,
                    residual: text(flow.residual)?,
                    control_flow: text(flow.control_flow)?,
                })
            };
        }

        Some(match protocol {
            Protocol::NamedErrorResult(_) => {
                shape!(NamedErrorResult, named_error_result_propagation)
            }
            Protocol::BorrowedResult => shape!(BorrowedResult, borrowed_result_propagation),
            Protocol::ScalarResult => shape!(ScalarResult, scalar_result_propagation),
            _ => shape!(ProductResult, product_result_propagation),
        })
    }

    /// `Result<(), E>` propagated from a function returning exactly it.
    fn same_error_shape(
        &self,
        input: &str,
        control_flow: &str,
    ) -> Option<crate::result_propagation::capture::Shape> {
        let (ok, error) = self.payloads(input)?;
        let unit = |id: &str| self.reader.ty(id).is_ok_and(|ty| ty.kind == TypeKind::Unit);
        if !unit(ok) || input != self.body.function.signature.result {
            return None;
        }
        let [residual, value] = self.arguments(control_flow)? else {
            return None;
        };
        let (never, same_error) = self.payloads(residual)?;
        if !unit(value) || !self.never(never) || error != same_error {
            return None;
        }

        Some(crate::result_propagation::capture::Shape {
            result: self.spelling(input)?,
            error: self.spelling(error)?,
            residual: self.spelling(residual)?,
            control_flow: self.spelling(control_flow)?,
        })
    }

    /// `Option<&u8>` propagated from a function returning an Option.
    fn option_shape(
        &self,
        input: &str,
        control_flow: &str,
    ) -> Option<crate::option_propagation::capture::Shape> {
        let [value] = self
            .reader
            .diagnostic_arguments(self.reader.ty(input).ok()?, "Option")?
        else {
            return None;
        };
        let shared_byte = self.reader.ty(value).is_ok_and(|ty| {
            ty.kind == TypeKind::Reference
                && ty.mode.as_deref() == Some("shared")
                && self.reader.integer(ty.target.as_deref(), "u8")
        });
        let output = self.body.function.signature.result.as_str();
        let [output_payload] = self
            .reader
            .diagnostic_arguments(self.reader.ty(output).ok()?, "Option")?
        else {
            return None;
        };
        let [residual, payload] = self.arguments(control_flow)? else {
            return None;
        };
        if !shared_byte || payload != value {
            return None;
        }

        Some(crate::option_propagation::capture::Shape {
            input: self.spelling(input)?,
            output: self.spelling(output)?,
            payload: self.spelling(value)?,
            output_payload: self.spelling(output_payload)?,
            residual: self.spelling(residual)?,
            control_flow: self.spelling(control_flow)?,
        })
    }

    /// The `Result<!, E>` residual of a `ControlFlow` continuing with `payload`.
    fn checked_result_residual(
        &self,
        control_flow: &'a str,
        payload: &str,
        error: &str,
    ) -> Option<&'a str> {
        let ty = self.reader.ty(control_flow).ok()?;
        let definition = self
            .reader
            .tables
            .definition(ty.definition.as_deref()?)
            .ok()?;
        let [residual, value] = ty.arguments.as_deref()? else {
            return None;
        };
        let control = definition.name.ends_with("ops::ControlFlow")
            && identity(definition, "crate_name") == Some("core");
        let (never, same_error) = self.payloads(residual)?;

        (control && value == payload && self.never(never) && same_error == error)
            .then_some(residual.as_str())
    }

    fn named_error_payloads(&self, mode: Mode, input: &str, output: &str) -> bool {
        let pair = |id: &str, first: &str| {
            matches!(self.items(id).as_deref(), Some([value, size])
                if self.reader.integer(Some(value), first) && self.reader.integer(Some(size), "usize"))
        };
        let unit = self
            .reader
            .ty(input)
            .is_ok_and(|ty| ty.kind == TypeKind::Unit);
        match mode {
            Mode::UnitToReader => unit && pair(output, "u32"),
            Mode::UnitToU16Reader => unit && pair(output, "u16"),
            Mode::ReaderToScalar => {
                pair(input, "u32") && self.reader.integer(Some(output), "usize")
            }
            Mode::ReaderToCopyProduct => pair(input, "u32") && self.copy_product(output),
            Mode::IdToReader => {
                self.local_plain_struct(input)
                    && matches!(self.items(output).as_deref(), Some([id, size])
                        if id == input && self.reader.integer(Some(size), "usize"))
            }
        }
    }

    /// `(S, usize)` where `S` is a local struct of three bools.
    fn copy_product(&self, output: &str) -> bool {
        let Some(items) = self.items(output) else {
            return false;
        };
        let [product, count] = items.as_slice() else {
            return false;
        };
        let fields = self
            .reader
            .ty(product)
            .ok()
            .and_then(|ty| ty.fields.as_deref())
            .unwrap_or_default();
        self.reader.integer(Some(count), "usize")
            && self.local_plain_struct(product)
            && fields.len() == 3
            && fields.iter().all(|field| {
                self.reader
                    .ty(&field.ty)
                    .is_ok_and(|ty| ty.kind == TypeKind::Bool)
            })
    }

    fn local_plain_struct(&self, id: &str) -> bool {
        let Ok(ty) = self.reader.ty(id) else {
            return false;
        };
        let definition = ty
            .definition
            .as_deref()
            .and_then(|definition| self.reader.tables.definition(definition).ok());
        ty.kind == TypeKind::Record
            && ty.arguments.as_deref().unwrap_or_default().is_empty()
            && !ty.modifiers.iter().any(|m| m == "rust:lifetime-arguments")
            && definition.is_some_and(|definition| {
                definition.scope == Scope::Unit
                    && identity(definition, "def_kind") == Some("Struct")
            })
    }

    fn byte_slice(&self, id: &str) -> bool {
        self.reader.ty(id).is_ok_and(|ty| {
            ty.kind == TypeKind::Reference
                && ty.mode.as_deref() == Some("shared")
                && ty
                    .target
                    .as_deref()
                    .and_then(|target| self.reader.ty(target).ok())
                    .is_some_and(|slice| {
                        slice.kind == TypeKind::Sequence
                            && slice.fixed != Some(true)
                            && self.reader.integer(slice.element.as_deref(), "u8")
                    })
        })
    }

    fn payloads(&self, id: &str) -> Option<(&'a str, &'a str)> {
        match self
            .reader
            .diagnostic_arguments(self.reader.ty(id).ok()?, "Result")?
        {
            [ok, error] => Some((ok.as_str(), error.as_str())),
            _ => None,
        }
    }

    fn arguments(&self, id: &str) -> Option<&'a [String]> {
        self.reader.tables.ty(id).ok()?.arguments.as_deref()
    }

    fn items(&self, id: &str) -> Option<Vec<String>> {
        let ty: &Type = self.reader.ty(id).ok()?;
        (ty.kind == TypeKind::Tuple).then(|| ty.items.clone().unwrap_or_default())
    }

    fn never(&self, id: &str) -> bool {
        self.reader
            .ty(id)
            .is_ok_and(|ty| ty.kind == TypeKind::Bottom)
    }
}

// rstyle-allow(mutation_per_target): builds the erased spelling character by character
/// A type's spelling with its lifetimes erased.
fn erased(spelling: &str) -> String {
    let mut erased = String::with_capacity(spelling.len());
    let mut chars = spelling.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\'' {
            erased.push(character);
            continue;
        }
        while chars
            .peek()
            .is_some_and(|next| next.is_alphanumeric() || *next == '_')
        {
            chars.next();
        }
        if chars.peek() == Some(&' ') {
            chars.next();
        }
    }
    erased
}
