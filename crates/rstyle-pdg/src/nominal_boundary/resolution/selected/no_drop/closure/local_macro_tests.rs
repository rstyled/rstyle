//! Original source expectations exercise expansion through the exhaustive visitor.
fn inspect(text: &str) -> Result<(), String> {
    let file = syn::parse_file(text).unwrap();
    let items = file.items.iter().collect::<Vec<_>>();
    super::inspect_ownership(&items, &vec!["Error".into()])
}

fn inspect_extra(extra: &str) -> Result<(), String> {
    inspect(&format!("struct Error(()); {extra}"))
}

#[test]
fn original_range_and_floor_templates_preserve_complete_expression_bodies() {
    let definitions = include_str!("../macros/ownership/fixtures/internal_macros.rs.txt");
    let source = format!(
        "{definitions}\nstruct Error(());\n\
         fn floor(value: i32) {{ div_floor!(value, 3); }}\n\
         fn checked(value: i32) {{ ensure_ranged!(Year: value); }}\n\
         fn named(value: i32) {{ ensure_ranged!(Year: value (\"year\")); }}\n\
         fn scaled(value: i32) {{ ensure_ranged!(Year: value as i64 * 2); }}"
    );
    inspect(&source).unwrap();
}

#[test]
fn original_expression_capture_cannot_hide_a_nested_destructor() {
    let definitions = include_str!("../macros/ownership/fixtures/internal_macros.rs.txt");
    let source = format!(
        "{definitions}\nstruct Error(());\n\
         fn floor(value: i32) {{\n\
             div_floor!({{ impl Drop for Error {{ fn drop(&mut self) {{}} }} value }}, 3);\n\
         }}"
    );
    assert!(inspect(&source).is_err());
}

#[test]
fn ordered_rule_selection_does_not_skip_an_earlier_destructor_arm() {
    let definition = r#"
        macro_rules! choose {
            (danger) => { impl Drop for Error { fn drop(&mut self) {} } };
            ($name:ident) => {};
        }
    "#;
    inspect_extra(&format!("{definition} choose!(safe);")).unwrap();
    assert!(inspect_extra(&format!("{definition} choose!(danger);")).is_err());
}

#[test]
fn ordered_rule_selection_does_not_erase_a_later_destructor_arm() {
    let definition = r#"
        macro_rules! choose {
            (safe) => {};
            ($name:ident) => { impl Drop for Error { fn drop(&mut self) {} } };
        }
    "#;
    inspect_extra(&format!("{definition} choose!(safe);")).unwrap();
    assert!(inspect_extra(&format!("{definition} choose!(danger);")).is_err());
}

#[test]
fn historical_cleanup_grammar_does_not_acquire_ordered_rule_support() {
    let file = syn::parse_file(
        "struct Error(()); macro_rules! hidden { () => { fn harmless() {} }; \
         ($name:ident) => { fn $name() {} }; } hidden!();",
    )
    .unwrap();
    let items = file.items.iter().collect::<Vec<_>>();
    let target = vec!["Error".into()];
    super::inspect_ownership(&items, &target).unwrap();
    assert!(super::inspect_scalar_error(&items, &target).is_err());
}

#[test]
fn forwarded_expression_opacity_survives_the_block_parser_and_nested_expansion() {
    let definition = r#"
        macro_rules! inner {
            (0) => { () };
            ($value:expr) => {{ impl Drop for Error { fn drop(&mut self) {} } }};
        }
        macro_rules! outer { ($value:expr) => { inner!($value); }; }
    "#;
    inspect_extra(&format!("{definition} fn direct() {{ inner!(0); }}")).unwrap();
    assert!(inspect_extra(&format!("{definition} fn forwarded() {{ outer!(0); }}")).is_err());
}

#[test]
fn selected_output_items_blocks_and_recursive_nested_items_are_all_inspected() {
    for template in [
        "impl Drop for Error { fn drop(&mut self) {} }",
        "{ impl Drop for Error { fn drop(&mut self) {} } }",
        "{ const _: () = { impl Drop for Error { fn drop(&mut self) {} } }; }",
    ] {
        let source = format!(
            "macro_rules! generated {{ () => {{ {template} }}; }} fn f() {{ generated!(); }}"
        );
        assert!(inspect_extra(&source).is_err(), "{template}");
    }
    inspect_extra(
        r#"
        struct Other;
        macro_rules! generated { () => {{ impl Drop for Other { fn drop(&mut self) {} } }}; }
        fn f() { generated!(); }
    "#,
    )
    .unwrap();
}

#[test]
fn repeated_type_arguments_preserve_every_emitted_destructor_target() {
    let definition = r#"
        struct Other;
        macro_rules! generated {
            ($($owner:ty),* $(,)?) => { $(impl Drop for $owner { fn drop(&mut self) {} })* };
        }
    "#;
    inspect_extra(&format!("{definition} generated!(Other,);")).unwrap();
    assert!(inspect_extra(&format!("{definition} generated!(Other, Error,);")).is_err());
}

#[test]
fn nested_textual_redefinition_changes_the_selected_body() {
    let definition = r#"macro_rules! chosen { () => { () }; }"#;
    inspect_extra(&format!("{definition} fn f() {{ chosen!(); }}")).unwrap();
    let shadow = r#"
        fn f() {
            macro_rules! chosen { () => {{ impl Drop for Error { fn drop(&mut self) {} } }}; }
            chosen!();
        }
    "#;
    assert!(inspect_extra(&format!("{definition} {shadow}")).is_err());
}

#[test]
fn nested_imports_unknown_macros_and_globs_do_not_supply_expansion_authority() {
    for body in [
        "use external::chosen; chosen!();",
        "use external::chosen as renamed; renamed!();",
        "use external::*; chosen!();",
        "unknown!();",
        "crate::unknown::chosen!();",
    ] {
        let source = format!("macro_rules! chosen {{ () => {{ () }}; }} fn f() {{ {body} }}");
        assert!(inspect_extra(&source).is_err(), "{body}");
    }
}

#[test]
fn generated_imports_and_macro_attributes_remain_visible_to_the_scope_visitor() {
    for output in [
        "use external::*; assert!(true);",
        "#[custom] struct Generated;",
        "#[custom] macro_rules! hidden { () => {} } hidden!();",
    ] {
        let source = format!(
            "macro_rules! generated {{ () => {{ {output} }}; }} fn f() {{ generated!(); }}"
        );
        assert!(inspect_extra(&source).is_err(), "{output}");
    }
}

#[test]
fn recursive_expansion_exhaustion_remains_unproved() {
    let error =
        inspect_extra("macro_rules! recurse { () => { recurse!(); }; } fn f() { recurse!(); }")
            .unwrap_err();
    assert!(error.contains("nesting"), "{error}");
}
