use regex_automata::{util::prefilter::Prefilter, MatchKind, Span};

fn check(pre: &Prefilter, needles: &[[u8; 1]], hay: &[u8], start: usize, end: usize) {
    let expected = if start >= hay.len() {
        None
    } else if needles.iter().any(|needle| needle[0] == hay[start]) {
        Some((start, start + 1))
    } else {
        None
    };
    let observed = pre.prefix(hay, Span { start, end });
    assert_eq!(
        observed.map(|span| (span.start, span.end)),
        expected,
        "needles={needles:?}, hay={hay:?}, start={start}, end={end}"
    );
}

fn main() {
    let enabled = std::env::args().nth(1).unwrap() == "default";
    let mut cases = 0u64;
    let mut constructors = 0u64;
    let mut incidental_queries = 0u64;
    for kind in [MatchKind::All, MatchKind::LeftmostFirst] {
        let none: [&[u8]; 0] = [];
        assert!(Prefilter::new(kind, &none).is_none());
        assert!(Prefilter::new(kind, &[b"".as_slice()]).is_none());
        constructors += 2;
        for needle in 0..=255u16 {
            let n = needle as u8;
            for needles in [
                vec![[n]],
                vec![[n], [n]],
                vec![[n], [n], [n]],
                vec![[n], [n.wrapping_add(1)]],
                vec![[n], [n.wrapping_add(1)], [n]],
            ] {
                let Some(pre) = Prefilter::new(kind, &needles) else {
                    assert!(!enabled, "default constructor unexpectedly unavailable");
                    constructors += 1;
                    continue;
                };
                assert!(enabled, "no-default constructor unexpectedly available");
                let label = match needles.len() {
                    1 => "Memchr",
                    2 => "Memchr2",
                    3 => "Memchr3",
                    _ => unreachable!(),
                };
                // Derived Debug exposes the actual selected concrete trait object.
                assert!(
                    format!("{pre:?}").contains(&format!("pre: {label}(")),
                    "wrong concrete dispatch: {pre:?}"
                );
                constructors += 1;
                // Post-measurement audit of incidental neighboring admissions.
                // These are not additions to the predeclared prefix selection.
                assert_eq!(pre.memory_usage(), 0);
                assert!(pre.is_fast());
                incidental_queries += 2;
                for byte in 0..=255u16 {
                    let hay = [byte as u8];
                    for start in [0, 1, 2, usize::MAX] {
                        for end in [0, 1, usize::MAX] {
                            check(&pre, &needles, &hay, start, end);
                            cases += 1;
                        }
                    }
                }
                for hay in [&[][..], &[0, n, 255][..]] {
                    for start in [0, 1, 2, 3, usize::MAX] {
                        for end in [0, 1, 3, usize::MAX] {
                            check(&pre, &needles, hay, start, end);
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(constructors, 2 * (2 + 256 * 5));
    assert_eq!(
        cases,
        if enabled {
            2 * 256 * 5 * (256 * 4 * 3 + 2 * 5 * 4)
        } else {
            0
        }
    );
    assert_eq!(
        incidental_queries,
        if enabled { 2 * 256 * 5 * 2 } else { 0 }
    );
    println!("{{\"prefix_cases\":{cases},\"constructor_cases\":{constructors},\"incidental_queries\":{incidental_queries}}}");
}
