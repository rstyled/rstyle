//! Complete source proofs, independently distinguishing module and body scope.
use super::*;

const LOCAL: &str = "fn selected() {}";
const OWNER: &str = "fn local_shadow() { struct Option; fn selected() {} }";

struct Reader {
    tree: TypeTree,
    resolution: Resolution,
    facts: HirFnFacts,
    origins: Origins,
}

impl Reader {
    fn new(spelling: &str, extra: &str) -> Self {
        let mut tree = reader_tree(spelling);
        let declaration: &str = tree.nominal_parts().unwrap().1;
        let source = format!("{declaration}\n{extra}\n{OWNER}");
        let resolution = resolution(&source, &mut tree);
        let mut facts = facts(&tree);
        let start = source.rfind(LOCAL).unwrap() as u32;
        facts.fn_span = crate::ByteSpan {
            start,
            end: start + LOCAL.len() as u32,
        };
        facts.source.bytes = LOCAL.into();
        facts.source.digest = facts.source.bytes.identity().into();
        facts.build.frontend_options.insert(
            crate::nominal_boundary::CONTEXT_KEY.into(),
            serde_json::to_value(&resolution.environment.census().context).unwrap(),
        );
        facts.build.frontend_options.insert(
            "source_file".into(),
            serde_json::json!(resolution.environment.census().files[0].path),
        );
        let origins = origins(&resolution, &facts);
        Self {
            tree,
            resolution,
            facts,
            origins,
        }
    }

    fn verify(&self) -> Result<CheckedNoDrop, String> {
        verify(&self.facts, &self.tree, &self.resolution, &self.origins)
    }
}

fn reader_tree(spelling: &str) -> TypeTree {
    let selection: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../eval/coverage-development/line-reader/selection.json"
    )))
    .unwrap();
    let original = selection["owners"][0]["source"].as_str().unwrap();
    let declaration = original.replace("Option<usize>", spelling);
    let mut tree = fixture();
    let TypeTree::Struct {
        nominal,
        source,
        fields,
        ..
    } = &mut tree
    else {
        unreachable!()
    };
    nominal.definition_path = "LineReader".into();
    *source = declaration.into();
    let mut remaining = fields[0].clone();
    remaining.name = "remaining".into();
    remaining.declaration = "&'i [u8]".into();
    let mut width = fields[3].clone();
    width.position = 1;
    width.name = "line_width".into();
    width.declaration = spelling.into();
    width.ty = TypeTree::Option(Box::new(TypeTree::Scalar("usize".into())));
    *fields = vec![remaining, width];
    tree
}

#[test]
fn frozen_reader_full_cleanup_proof_uses_declaration_scope_despite_local_shadow() {
    let reader = Reader::new("Option<usize>", "");
    let checked = reader.verify().unwrap();
    assert_eq!(checked.nominal().definition_path, "LineReader");
    assert!(reader.origins.declarations[0].builtin_copy.is_none());
    let local = reader.resolution.environment.verify_option_type(
        &reader.facts,
        &syn::parse_str("Option<usize>").unwrap(),
        "usize",
    );
    assert!(local.unwrap_err().contains("Option path"));
}

#[test]
fn complete_reader_proof_accepts_direct_core_alias_and_qualified_option() {
    for (spelling, extra) in [
        ("Maybe<usize>", "use core::option::Option as Maybe;"),
        ("core::option::Option<usize>", ""),
        ("std::option::Option<usize>", ""),
    ] {
        let reader = Reader::new(spelling, extra);
        let result = reader.verify();
        assert!(result.is_ok(), "{spelling}: {extra}: {result:?}");
    }
}

#[test]
fn resealed_module_shadows_do_not_mint_reader_cleanup_authority() {
    for (spelling, extra) in [
        ("Option<usize>", "struct Option<T>(T);"),
        ("Option<usize>", "use foreign::Option;"),
        ("Option<usize>", "type usize = u32;"),
        ("core::option::Option<usize>", "mod core {}"),
    ] {
        let reader = Reader::new(spelling, extra);
        let error = reader.verify().unwrap_err();
        assert!(
            error.contains("Option")
                || error.contains("import")
                || error.contains("ambiguous")
                || error.contains("Primitive spelling"),
            "{spelling}: {extra}: {error}"
        );
    }
}

#[test]
fn ambiguous_option_imports_fail_before_a_cleanup_environment_exists() {
    let mut census = (*environment::tests::census()).clone();
    census.files[0].bytes = "use core::option::Option; use foreign::Option;".into();
    census.files[0].digest = census.files[0].bytes.identity().into();
    let result = Environment::with_options(Arc::new(census), true, Some(&[]), false);
    assert!(matches!(result, Err(error) if error.contains("Ambiguous named import")));
}

#[test]
fn resealed_reader_source_rejects_cleanup_in_neighbors_and_expansions() {
    for extra in [
        "impl Drop for LineReader<'_> { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for LineReader<'_> { fn drop(&mut self) {} } }",
        "macro_rules! hidden { () => { impl Drop for LineReader<'_> { fn drop(&mut self) {} } }; } hidden!();",
        "use foreign::Cleanup; impl Cleanup for LineReader<'_> {}",
    ] {
        let reader = Reader::new("Option<usize>", extra);
        let error = reader.verify().unwrap_err();
        assert!(
            error.contains("possible destructor") || error.contains("lexical macro expansion"),
            "{extra}: {error}"
        );
    }
}
