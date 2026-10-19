use super::*;

#[test]
fn correspondence_erases_only_transparent_parentheses_and_comments() {
    for (before, after) in [
        ("(", ")"),
        ("(/* ( */ (", ") /* ) */)"),
        ("(// leading\n", " // trailing\n)"),
    ] {
        let source = format!("{before}1 | 2{after}");
        let syntax = ByteSpan {
            start: before.len() as u32,
            end: (before.len() + 5) as u32,
        };
        let hir = ByteSpan {
            start: 0,
            end: source.len() as u32,
        };
        assert!(corresponding_span(&source, syntax, hir), "{source}");
    }
    for (before, after) in [
        ("[/* comment */", "]"),
        ("f(/* comment */", ")"),
        ("(/* comment */", ",)"),
        ("{/* comment */", "}"),
        ("((/* comment */", ")"),
    ] {
        assert!(
            !commented_parentheses(before, after),
            "{before}value{after}"
        );
    }
}
