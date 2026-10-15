//! Checked work-budget accounting shared by restoration and optional exports.
pub(crate) fn charge(remaining: &mut usize, bytes: usize, reason: &str) -> Result<(), String> {
    *remaining = remaining.checked_sub(bytes).ok_or(reason)?;
    Ok(())
}

/// Count the exact wire encoding without allocating a serialized copy.
pub(crate) fn serialized_len(value: &impl serde::Serialize) -> Result<usize, serde_json::Error> {
    let mut counter = ByteCount::default();
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.0)
}

#[derive(Default)]
struct ByteCount(usize);

impl std::io::Write for ByteCount {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wire_size_counts_unicode_and_escapes_exactly_without_retaining_bytes() {
        let value =
            serde_json::json!({"source": "\u{feff}// π\r\n\"\\", "values": [null, 10, true]});
        assert_eq!(
            super::serialized_len(&value).unwrap(),
            serde_json::to_vec(&value).unwrap().len()
        );
        assert_eq!(super::serialized_len(&Vec::<String>::new()).unwrap(), 2);
    }
}
