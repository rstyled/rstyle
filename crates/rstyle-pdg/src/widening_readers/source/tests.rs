use super::*;

const ORIGINAL: &str = r#"
pub(crate) fn try_read_u32_as_usize(slice: &[u8], what: &'static str)
    -> Result<(usize, usize), DeserializeError>
{
    try_read_u32(slice, what).and_then(|(n, nr)| {
        usize::try_from(n)
            .map(|n| (n, nr))
            .map_err(|_| DeserializeError::invalid_usize(what))
    })
}
"#;

fn check(source: &str) -> bool {
    let function = syn::parse_str(source).unwrap();
    syntax(&function).is_ok()
}

#[test]
fn original_and_alpha_renamed_roles_are_complete() {
    let function = syn::parse_str(ORIGINAL).unwrap();
    let roles = syntax(&function).unwrap();
    assert_eq!(roles.formals[0].ident, "slice");
    assert_eq!(roles.formals[1].ident, "what");
    assert_eq!(roles.decoded.ident, "n");
    assert_eq!(roles.count.ident, "nr");
    assert_eq!(roles.converted.ident, "n");
    assert_eq!(roles.and_then.method, "and_then");
    assert_eq!(roles.map.method, "map");
    assert_eq!(roles.map_err.method, "map_err");
    assert_eq!(roles.reader.args.len(), 2);
    assert_eq!(roles.conversion.args.len(), 1);
    assert_eq!(roles.constructor.args.len(), 1);
    assert_eq!(roles.tuple.elems.len(), 2);
    assert_eq!(roles.continuation.inputs.len(), 1);
    assert_eq!(roles.success.inputs.len(), 1);
    assert_eq!(roles.failure.inputs.len(), 1);
    assert!(roles.error_owner.path.is_ident("DeserializeError"));

    assert!(check(ORIGINAL));
    assert!(check(
        &ORIGINAL
            .replace("slice", "bytes")
            .replace("what", "label")
            .replace("nr", "consumed")
            .replace("|n| (n, consumed)", "|value| (value, consumed)")
    ));
    assert!(check(&ORIGINAL.replace("DeserializeError", "ErrorAlias")));
}

#[test]
fn changed_dataflow_and_additional_work_are_rejected() {
    for (before, after) in [
        ("&[u8]", "&[u16]"),
        ("&'static str", "&str"),
        ("(usize, usize)", "(u32, usize)"),
        ("(n, nr))", "(nr, n))"),
        ("(n, nr))", "(n, n))"),
        ("(n, nr))", "(n, 4))"),
        ("invalid_usize(what)", "invalid_usize(\"wrong\")"),
        ("invalid_usize(what)", "buffer_too_small(what)"),
        (
            "try_read_u32(slice, what)",
            "try_read_u32(slice, \"wrong\")",
        ),
        ("try_read_u32(slice, what)", "try_read_u16(slice, what)"),
        ("usize::try_from(n)", "usize::try_from(nr)"),
        ("usize::try_from(n)", "{ effect(); usize::try_from(n) }"),
        ("usize::try_from(n)", "effect(); usize::try_from(n)"),
        (".map(|n|", ".and_then(|n|"),
        ("|(n, nr)|", "|(what, nr)|"),
        ("|(n, nr)|", "|(n, what)|"),
        ("|n| (n, nr)", "|nr| (nr, nr)"),
    ] {
        assert!(
            !check(&ORIGINAL.replace(before, after)),
            "{before} -> {after}"
        );
    }
}

#[test]
fn qualifiers_patterns_and_hidden_work_are_rejected() {
    for (before, after) in [
        ("pub(crate) fn", "#[inline] pub(crate) fn"),
        ("pub(crate) fn", "pub(crate) async fn"),
        ("slice: &[u8]", "mut slice: &[u8]"),
        ("|(n, nr)|", "move |(n, nr)|"),
        ("|(n, nr)|", "|(mut n, nr)|"),
        ("|n|", "|n: usize|"),
        ("|_|", "|error|"),
        ("usize::try_from", "std::usize::try_from"),
        ("usize::try_from", "<usize as TryFrom<u32>>::try_from"),
        (".map(|n|", ".map::<usize>(|n|"),
        (
            "try_read_u32(slice, what)",
            "wire::try_read_u32(slice, what)",
        ),
        ("try_read_u32(slice, what)", "try_read_u32(slice, what, 4)"),
        ("invalid_usize(what)", "invalid_usize(what, 4)"),
        ("usize::try_from(n)", "usize::try_from(n, 4)"),
        ("    try_read_u32", "    effect(); try_read_u32"),
    ] {
        assert!(
            !check(&ORIGINAL.replace(before, after)),
            "{before} -> {after}"
        );
    }
}
