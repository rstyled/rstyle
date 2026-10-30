//! Legacy physical-line selection over the already checked original file.
use super::Availability;
use crate::{ByteSpan, HirFnFacts, SourceText, nominal_boundary::census};
use std::sync::Arc;

const LOOKBACK_LINES: usize = 40;

// Called only after source ownership and the original signature have checked.
pub(super) fn inspect(facts: &HirFnFacts) -> Result<Availability<SourceText>, String> {
    let options = &facts.build.frontend_options;
    let census = options.data::<Arc<census::Census>>()?;
    let path = options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Checked function source path is unavailable")?;
    let file = census
        .files
        .iter()
        .find(|file| file.path.to_str() == Some(path))
        .ok_or("Checked function source file is unavailable")?;

    let start = facts
        .fn_span
        .start
        .checked_add(facts.signature.sig_span.start)
        .ok_or("Original signature start exceeds source bounds")?;
    let span = selected_span(&file.bytes, start as usize)?;
    SourceText::from_snapshot(file.bytes.clone(), span).map(Availability::Known)
}

fn selected_span(source: &str, signature_start: usize) -> Result<ByteSpan, String> {
    let prefix = source
        .get(..signature_start)
        .ok_or("Original signature start is not a source byte boundary")?;
    let Some(line_start) = prefix.rfind('\n') else {
        return Ok(ByteSpan { start: 0, end: 0 });
    };
    let preceding = &prefix[..line_start];
    let selected = preceding
        .rsplit('\n')
        .take(LOOKBACK_LINES)
        .map(str::trim)
        .find(|line| !line.starts_with("#[") && !line.starts_with("///"))
        .unwrap_or("");
    if selected.is_empty() {
        return Ok(ByteSpan { start: 0, end: 0 });
    }

    let start = selected.as_ptr() as usize - source.as_ptr() as usize;
    Ok(ByteSpan {
        start: u32::try_from(start).map_err(|_| "Selected line exceeds source bounds")?,
        end: u32::try_from(start + selected.len())
            .map_err(|_| "Selected line exceeds source bounds")?,
    })
}

#[cfg(test)]
mod tests;
