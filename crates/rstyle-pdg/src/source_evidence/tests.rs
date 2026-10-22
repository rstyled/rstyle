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

#[test]
fn parser_byte_ranges_preserve_original_utf8_offsets() {
    for prefix in ["", "\u{feff}"] {
        for newline in ["\n", "\r\n"] {
            let bytes = format!(
                "{prefix}// π before the function{newline}pub fn café(値: u8) -> u8 {{{newline}    // 🦀 before the value{newline}    値{newline}}}{newline}"
            );
            let file = syn::parse_file(&bytes).unwrap();
            let mut check = CheckSpans {
                bytes: &bytes,
                count: 0,
            };
            check.visit_file(&file);
            assert!(check.count > 0);
            let span = original_span(&bytes, file.items[0].span());
            assert_eq!(
                &bytes[span.start as usize..span.end as usize],
                format!(
                    "pub fn café(値: u8) -> u8 {{{newline}    // 🦀 before the value{newline}    値{newline}}}"
                )
            );
        }
    }
}

struct CheckSpans<'a> {
    bytes: &'a str,
    count: usize,
}

impl<'ast> Visit<'ast> for CheckSpans<'_> {
    fn visit_span(&mut self, span: &proc_macro2::Span) {
        let source = self.bytes.strip_prefix('\u{feff}').unwrap_or(self.bytes);
        let prefix = (self.bytes.len() - source.len()) as u32;
        assert_eq!(
            original_span(self.bytes, *span),
            ByteSpan {
                start: prefix + byte_offset(source, span.start()) as u32,
                end: prefix + byte_offset(source, span.end()) as u32,
            }
        );
        self.count += 1;
    }
}
