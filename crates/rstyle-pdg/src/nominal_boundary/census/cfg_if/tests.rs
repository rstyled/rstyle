use super::*;
use syn::spanned::Spanned;

fn selected(source: &str, cfgs: &[String]) -> Result<Vec<syn::Item>, String> {
    let invocation = syn::parse_str(source).map_err(|error: syn::Error| error.to_string())?;
    select(&invocation, &Configuration::new(cfgs))
}

#[test]
fn exact_definition_checks_every_arm_and_rejects_name_only_authority() {
    let original: syn::ItemMacro = syn::parse_str(DEFINITION).unwrap();
    verify_definition(&original).unwrap();
    for source in [
        DEFINITION.replace("macro_rules! cfg_if", "macro_rules! other"),
        DEFINITION.replace("not(any($($not),*))", "any($($not),*)"),
        DEFINITION.replace(
            "(@__identity $($tokens:tt)*) => { $($tokens)* };",
            "(@__identity $($tokens:tt)*) => {};",
        ),
        format!("#[unknown]\n{DEFINITION}"),
        "macro_rules! cfg_if { ($($tokens:tt)*) => { mod hidden; }; }".into(),
    ] {
        let changed = syn::parse_str(&source).unwrap();
        assert!(verify_definition(&changed).is_err(), "{source}");
    }
}

#[test]
fn selected_branch_preserves_first_match_items_and_original_source_spans() {
    let source = "cfg_if! {\n if #[cfg(unix)] {\n mod first; use first::Value;\n } else if #[cfg(all())] { mod later; } else { mod last; }\n}";
    let items = selected(source, &["unix".into()]).unwrap();
    assert_eq!(items.len(), 2);
    let syn::Item::Mod(module) = &items[0] else {
        panic!("module lost");
    };
    assert_eq!(module.ident, "first");
    assert_eq!(module.span().start().line, 3);
    assert!(matches!(items[1], syn::Item::Use(_)));
    let items = selected(source, &[]).unwrap();
    let syn::Item::Mod(module) = &items[0] else {
        panic!("module lost");
    };
    assert_eq!(module.ident, "later");
}

#[test]
fn inactive_output_empty_branches_and_unselected_nonitems_remain_distinct() {
    for source in [
        "cfg_if! { if #[cfg(missing)] { mod absent; } }",
        "cfg_if! { if #[cfg(all())] {} else { mod absent; } }",
        "#[cfg(missing)] cfg_if! { if #[cfg(all())] { mod absent; } }",
        "cfg_if! { if #[cfg(missing)] { arbitrary + tokens } else {} }",
    ] {
        assert!(selected(source, &[]).unwrap().is_empty(), "{source}");
    }
    let items = selected(
        "cfg_if! { if #[cfg(missing)] {} else { mod fallback; } }",
        &[],
    )
    .unwrap();
    assert!(matches!(&items[0], syn::Item::Mod(module) if module.ident == "fallback"));
}

#[test]
fn malformed_unknown_and_partial_selected_expansions_fail_closed() {
    for source in [
        "cfg_if! { @__identity mod hidden; }",
        "cfg_if! { if #[cfg(unknown())] { mod unknown; } }",
        "cfg_if! { if #[cfg(all())] {} else if #[cfg(unknown())] {} }",
        "cfg_if! { if #[cfg(all())] { mod good; const BAD: u8 = 0; } }",
        "cfg_if! { if #[cfg(all())] { mod good; other!(); } }",
        "cfg_if! { if #[cfg(all())] { mod good; fn hidden() {} } }",
        "cfg_if! { if #[cfg(all())] { mod good; } else {} trailing }",
        "cfg_if! { if #[cfg(all())] #[cfg(unix)] { mod good; } }",
        "cfg_if! { if #[cfg_attr(unix, cfg(all()))] { mod good; } }",
        "other::cfg_if! { if #[cfg(all())] { mod good; } }",
        "#[unknown] cfg_if! { if #[cfg(all())] { mod good; } }",
    ] {
        assert!(selected(source, &[]).is_err(), "{source}");
    }
}

#[test]
fn selection_reports_public_branch_position_and_final_else_presence() {
    for (source, index, has_else) in [
        (
            "cfg_if! { if #[cfg(all())] { mod first; } else { mod fallback; } }",
            Some(0),
            true,
        ),
        (
            "cfg_if! { if #[cfg(missing)] {} else if #[cfg(all())] { mod next; } }",
            Some(1),
            false,
        ),
        (
            "cfg_if! { if #[cfg(missing)] {} else { mod fallback; } }",
            Some(1),
            true,
        ),
        ("cfg_if! { if #[cfg(missing)] {} }", None, false),
        ("cfg_if! { if #[cfg(all())] {} }", Some(0), false),
    ] {
        let invocation = syn::parse_str(source).unwrap();
        let selection = select_with_index(&invocation, &Configuration::new(&[])).unwrap();
        assert_eq!(selection.index, index, "{source}");
        assert_eq!(selection.has_else, has_else, "{source}");
    }
}
