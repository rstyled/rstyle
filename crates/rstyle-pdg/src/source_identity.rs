//! Byte consistency only: source ownership and semantic evidence remain separate.
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Xxh3,
    LegacySha256,
}

impl Scheme {
    pub fn parse(claim: &str) -> Option<Self> {
        let (prefix, value) = claim.split_once(':')?;
        let (scheme, length) = match prefix {
            "xxh3-64" => (Self::Xxh3, 16),
            "sha256" => (Self::LegacySha256, 64),
            _ => return None,
        };
        (value.len() == length
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then_some(scheme)
    }

    pub fn digest(self, bytes: &[u8]) -> String {
        match self {
            Self::Xxh3 => crate::unit_store::transport_checksum(bytes),
            Self::LegacySha256 => crate::concrete_types::digest(bytes),
        }
    }
}

/// New routine source claims use the existing noncryptographic transport checksum.
pub fn checksum(bytes: &[u8]) -> String {
    crate::unit_store::transport_checksum(bytes)
}

/// Actual values owned by immutable bytes; never deserialized from wire claims.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo {
    legacy: OnceLock<String>,
    checksum: OnceLock<String>,
}

impl Memo {
    pub(crate) fn legacy(&self, bytes: &[u8]) -> &str {
        self.actual(bytes, Scheme::LegacySha256)
    }

    pub(crate) fn actual(&self, bytes: &[u8], scheme: Scheme) -> &str {
        let slot = match scheme {
            Scheme::Xxh3 => &self.checksum,
            Scheme::LegacySha256 => &self.legacy,
        };
        slot.get_or_init(|| scheme.digest(bytes))
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.legacy.get().is_none() && self.checksum.get().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routine_checksum_does_not_compute_legacy_sha() {
        let memo = Memo::default();
        assert_eq!(
            memo.actual(b"abc", Scheme::Xxh3),
            "xxh3-64:78af5f94892f3950"
        );
        assert!(memo.legacy.get().is_none());
        assert_eq!(
            memo.actual(b"abc", Scheme::LegacySha256),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(memo.checksum.get().unwrap(), "xxh3-64:78af5f94892f3950");
    }

    #[test]
    fn both_formats_check_actual_bytes_and_reject_malformed_claims() {
        for bytes in [
            b"".as_slice(),
            b"abc",
            "\u{feff}fn café() {}\r\n".as_bytes(),
        ] {
            let source = crate::SourceSnapshot::from(std::str::from_utf8(bytes).unwrap());
            let changed = crate::SourceSnapshot::from("changed");
            for scheme in [Scheme::Xxh3, Scheme::LegacySha256] {
                let claim = scheme.digest(bytes);
                assert!(source.matches_identity(&claim));
                assert!(!changed.matches_identity(&claim));
            }
        }
        for claim in [
            "",
            "unknown:78af5f94892f3950",
            "xxh3-64:78af5f94892f395",
            "xxh3-64:78AF5F94892F3950",
            "xxh3-64:78af5f94892f39500",
            "sha256:78af5f94892f3950",
            "xxh3-64:78af5f94892f395z",
        ] {
            assert!(Scheme::parse(claim).is_none(), "{claim}");
            assert!(!crate::SourceSnapshot::from("abc").matches_identity(claim));
        }
    }
}
