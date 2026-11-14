//! Exists only to exercise the Rstyle webhook review. Do not merge.

/// Whether any name appears twice, by counting each name's matches: quadratic in `names`.
pub fn has_duplicate(names: &[String]) -> bool {
    for name in names {
        let mut seen = 0;
        for other in names {
            if name == other {
                seen += 1;
            }
        }
        if seen > 1 {
            return true;
        }
    }
    false
}
