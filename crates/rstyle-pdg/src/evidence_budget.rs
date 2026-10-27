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

/// Encode once, rejecting output before it exceeds the caller's retention budget.
pub(crate) fn encode_with_limit(
    value: &impl serde::Serialize,
    limit: usize,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Limited {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value)?;
    Ok(output.bytes)
}

struct Limited {
    bytes: Vec<u8>,
    limit: usize,
}

impl std::io::Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(std::io::Error::other(
                "Evidence encoding exceeds byte budget",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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

    #[test]
    fn bounded_encoding_preserves_exact_json_or_rejects_insufficient_budgets() {
        for value in [
            serde_json::json!(null),
            serde_json::json!("\u{feff}// π\r\n\"\\"),
            serde_json::json!({"nested": [true, false, 255, null], "text": "\t\u{0001}"}),
            serde_json::json!("x".repeat(4096)),
        ] {
            let expected = serde_json::to_vec(&value).unwrap();
            for limit in [0, expected.len() - 1, expected.len(), expected.len() + 7] {
                let actual = super::encode_with_limit(&value, limit);
                if limit < expected.len() {
                    assert!(
                        actual.is_err(),
                        "budget {limit} accepted {} bytes",
                        expected.len()
                    );
                } else {
                    assert_eq!(actual.unwrap(), expected);
                }
            }
        }
    }
}
