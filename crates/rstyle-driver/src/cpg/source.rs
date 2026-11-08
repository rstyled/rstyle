//! A source file's original bytes, restored from the compiler's normalization positions.
use rustc_span::SourceFile;

/// The file's original bytes, restored from the compiler's normalization positions.
pub(super) fn recover(file: &SourceFile) -> Option<String> {
    if !matches!(file.original_relative_byte_pos(file.start_pos).0, 0 | 3) {
        return None;
    }
    restore(
        file.src.as_ref()?,
        file.normalized_pos.iter().map(|p| (p.pos.0, p.diff)),
    )
}

/// rustc removes only a leading BOM and CR in CRLF. Recover recorded bytes,
/// never reopen a path that may have changed since the compiler read it.
fn restore(
    normalized: &str,
    changes: impl DoubleEndedIterator<Item = (u32, u32)>,
) -> Option<String> {
    let mut bytes = normalized.as_bytes().to_vec();
    for (position, difference) in changes.rev() {
        if position == 0 && difference == 3 {
            bytes.splice(0..0, [0xef, 0xbb, 0xbf]);
        } else {
            let position = position.checked_sub(1)? as usize;
            if bytes.get(position) != Some(&b'\n') {
                return None;
            }
            bytes.insert(position, b'\r');
        }
    }
    String::from_utf8(bytes).ok()
}
