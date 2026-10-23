use super::*;

fn node(kind: &str, span: std::ops::Range<u32>, local: u32, children: Vec<HirNode>) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local,
        },
        kind: kind.into(),
        span: ByteSpan {
            start: span.start,
            end: span.end,
        },
        snippet: String::new(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children,
    }
}

// Independent pre-index traversal, including alias and multiplicity semantics.
struct LegacyQuery<'a> {
    source: &'a str,
    kind: &'a str,
    span: ByteSpan,
}

impl LegacyQuery<'_> {
    fn collect(&self, node: &HirNode, matches: &mut Vec<HirRef>) {
        let same = node.kind == self.kind
            || (matches!(self.kind, "expr_call" | "expr_path") && node.kind == "expr_struct")
            || (self.kind == "pat_name"
                && matches!(node.kind.as_str(), "pat_binding" | "pat_constant"));
        if same && corresponding_span(self.source, self.span, node.span) {
            matches.push(node.hir.clone());
        }
        for child in &node.children {
            self.collect(child, matches);
        }
    }
}

fn overlapping_tree(source: &str) -> HirNode {
    let x = source.find('x').unwrap() as u32;
    node(
        "expr_struct",
        0..23,
        0,
        vec![
            node("expr_path", x..x + 1, 1, vec![]),
            node("pat_binding", 0..23, 2, vec![]),
            node("pat_constant", 3..5, 3, vec![]),
            node(
                "expr_call",
                std::ops::Range {
                    start: 100,
                    end: 90,
                },
                4,
                vec![],
            ),
            node("expr_path", 25..26, 5, vec![]),
            node("expr_path", 25..26, 5, vec![]),
            node("expr_struct", 0..source.len() as u32, 6, vec![]),
        ],
    )
}

#[test]
fn index_preserves_aliases_overlaps_comments_duplicates_and_malformed_spans() {
    let source = "( /* π */ (x) /* ) */ ) + x";
    let tree = overlapping_tree(source);
    let index = Candidates::new(&tree);
    for text in [source, "", "totally different source bytes", "\u{feff}π"] {
        for kind in [
            "expr_struct",
            "expr_call",
            "expr_path",
            "pat_name",
            "pat_binding",
            "pat_constant",
            "missing",
        ] {
            for start in (0..=30).chain([90, 100, 105]) {
                for end in (0..=30).chain([90, 100, 105]) {
                    let span = ByteSpan { start, end };
                    let mut expected = Vec::new();
                    LegacyQuery {
                        source: text,
                        kind,
                        span,
                    }
                    .collect(&tree, &mut expected);
                    let unique = (expected.len() == 1).then(|| expected[0].clone());
                    assert_eq!(
                        index.unique(text, kind, span),
                        unique,
                        "{text:?} {kind} {span:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn enclosing_commented_parentheses_remain_candidates() {
    let source = "(/* leading */ (x) /* trailing */)";
    let tree = node("expr_struct", 0..source.len() as u32, 7, vec![]);
    let x = source.find('x').unwrap() as u32;
    let span = ByteSpan {
        start: x,
        end: x + 1,
    };
    assert_eq!(
        Candidates::new(&tree).unique(source, "expr_path", span),
        Some(tree.hir)
    );
}

#[test]
fn disjoint_path_queries_visit_logarithmic_candidate_ranges() {
    for size in [128, 1024, 8192] {
        let tree = node(
            "expr_block",
            0..size * 2,
            0,
            (0..size)
                .map(|i| node("expr_path", i * 2..i * 2 + 1, i + 1, vec![]))
                .collect(),
        );
        let index = Candidates::new(&tree);
        let source = "x ".repeat(size as usize);
        let mut visited = 0;
        for i in 0..size {
            let span = ByteSpan {
                start: i * 2,
                end: i * 2 + 1,
            };
            let (result, work) = index.search(&source, "expr_path", span);
            visited += work;
            assert_eq!(result.unwrap().local, i + 1);
        }
        let per_query = 2 * (size.ilog2() as usize + 1) + 1;
        assert!(visited <= size as usize * per_query, "{size}: {visited}");
        assert!(visited < (size as usize).pow(2) / 4);
    }
}
