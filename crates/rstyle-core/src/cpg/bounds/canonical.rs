//! Bounds inputs compared up to the numbering of locals and loops.

/// The input's JSON with local and loop numbers renumbered by first appearance.
pub fn canonical_bounds(value: &serde_json::Value) -> String {
    let text = value.to_string();
    // Extent labels end `len(...)#N`; `#` alone also occurs in paths such as `{impl#2}`.
    let markers = ["\"Local\":", ")#", "for-index-", "for-limit-"];
    let mut numbers: std::collections::BTreeMap<String, usize> = Default::default();
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let found = markers
            .iter()
            .filter_map(|marker| rest.find(marker).map(|at| (at, *marker)))
            .min();
        let Some((at, marker)) = found else {
            out.push_str(rest);
            break;
        };
        let after = at + marker.len();
        out.push_str(&rest[..after]);
        let digits = rest[after..].bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            rest = &rest[after..];
            continue;
        }
        let number = &rest[after..after + digits];
        let next = numbers.len();
        let canonical = *numbers.entry(number.to_string()).or_insert(next);
        out.push_str(&canonical.to_string());
        rest = &rest[after + digits..];
    }

    out
}
