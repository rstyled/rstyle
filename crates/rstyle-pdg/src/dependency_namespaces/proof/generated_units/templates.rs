//! Original token coordinates, including impl/member sites retained for roster replay.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TemplateRole {
    Struct,
    InherentImpl,
    Per,
    PerT,
    IntegerImpl,
    IntegerValue,
    FloatImpl,
    FloatValue,
    DefaultImpl,
    DefaultType,
    DeriveDebug,
    DeriveClone,
    DeriveCopy,
    DeriveAttribute,
    PerTypeParameter,
    PerTOutput,
    PerTOpaque,
}

pub(super) fn map(
    file: &UnitFile,
    coordinates: &syntax::Coordinates,
) -> Result<Vec<(TemplateRole, FileSpan)>, ProofError> {
    specs()
        .into_iter()
        .map(|(role, text, offset, length)| {
            let start = syntax::TEMPLATE
                .find(text)
                .ok_or(ProofError::Contradictory(
                    "Missing independently specified generated-unit template role",
                ))?
                + offset;
            let end = start + length;
            let start = coordinates
                .iter()
                .filter(|(expected, _)| expected.start == start)
                .map(|(_, actual)| actual.start)
                .min();
            let end = coordinates
                .iter()
                .filter(|(expected, _)| expected.end == end)
                .map(|(_, actual)| actual.end)
                .max();
            let (Some(start), Some(end)) = (start, end) else {
                return Err(ProofError::Contradictory(
                    "Generated-unit role has no exact original token endpoints",
                ));
            };
            let start = u32::try_from(start).map_err(|_| {
                ProofError::Unsupported("Generated-unit template role exceeds source budget")
            })?;
            let end = u32::try_from(end).map_err(|_| {
                ProofError::Unsupported("Generated-unit template role exceeds source budget")
            })?;
            Ok((
                role,
                FileSpan {
                    path: file.path.clone(),
                    digest: file.digest.clone(),
                    span: crate::ByteSpan { start, end },
                },
            ))
        })
        .collect()
}

fn specs() -> Vec<(TemplateRole, &'static str, usize, usize)> {
    use TemplateRole::*;
    let per = "pub const fn per<T>(_larger: T) -> <T as DefaultOutput<Self>>::Output\n            where\n                T: MultipleOf<Self, T::Output> + DefaultOutput<Self> + Copy,";
    let per_t =
        "pub const fn per_t<Output>(larger: impl MultipleOf<Self, Output> + Copy) -> Output";
    let mut specs = [
        (Struct, "pub struct $t;"),
        (InherentImpl, "impl $t"),
        (Per, per),
        (PerT, per_t),
        (IntegerImpl, "impl MultipleOf<$t, $int_output> for $larger"),
        (IntegerValue, "const VALUE: $int_output = $int_value;"),
        (FloatImpl, "impl MultipleOf<$t, $float_output> for $larger"),
        (FloatValue, "const VALUE: $float_output = $float_value;"),
        (DefaultImpl, "impl DefaultOutput<$t> for $larger"),
        (DefaultType, "type Output = $default_output;"),
        (DeriveDebug, "Debug, Clone, Copy"),
        (DeriveClone, "Clone, Copy"),
        (DeriveCopy, "Copy)]"),
    ]
    .into_iter()
    .map(|(role, text)| (role, text, 0, text.len()))
    .collect::<Vec<_>>();
    specs[10].3 = "Debug".len();
    specs[11].3 = "Clone".len();
    specs[12].3 = "Copy".len();
    specs[9].3 = "type Output".len();
    specs.extend([
        (
            DeriveAttribute,
            "#[derive(Debug, Clone, Copy)]",
            0,
            "#[derive(Debug, Clone, Copy)]".len(),
        ),
        (PerTypeParameter, "per<T>", "per<".len(), 1),
        (PerTOutput, "per_t<Output>", "per_t<".len(), "Output".len()),
        (
            PerTOpaque,
            "impl MultipleOf<Self, Output> + Copy",
            0,
            "impl MultipleOf<Self, Output> + Copy".len(),
        ),
    ]);
    specs
}
