//! Original source fixtures and independently specified ownership footprints.
use super::*;

fn run(definition: &str, invocation: &str) -> Result<TokenStream, String> {
    let definition: syn::ItemMacro = syn::parse_str(definition).unwrap();
    let invocation: syn::ItemMacro = syn::parse_str(invocation).unwrap();
    expand(&definition, &invocation.mac, Context { edition: "2024" })
}

fn original(source: &str, name: &str, invocation: &str) -> TokenStream {
    let file = syn::parse_file(source).unwrap();
    let definition = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Macro(item) if item.ident.as_ref().is_some_and(|ident| ident == name) => {
                Some(item)
            }
            _ => None,
        })
        .unwrap();
    let invocation: syn::ItemMacro = syn::parse_str(invocation).unwrap();
    expand(definition, &invocation.mac, Context { edition: "2024" }).unwrap()
}

#[test]
fn original_internal_macros_keep_complete_selected_expression_and_statement_bodies() {
    let source = include_str!("fixtures/internal_macros.rs.txt");
    for (name, invocation) in [
        ("div_floor", "div_floor!(a + b, divisor);"),
        ("const_try", "const_try!(attempt());"),
        ("const_try_opt", "const_try_opt!(attempt());"),
        ("carry", "carry!(@most_once value, 0.. 60);"),
        ("carry", "carry!(@most_twice value, 0.. 60);"),
        ("carry", "carry!(@most_thrice value, -3.. 60);"),
        ("bug", "bug!(\"invalid state\");"),
        (
            "ensure_ranged",
            "ensure_ranged!(Ranged<i32, 0, 10>: value);",
        ),
        (
            "ensure_ranged",
            "ensure_ranged!(Ranged<i32, 0, 10>: value (\"name\"));",
        ),
        (
            "ensure_ranged",
            "ensure_ranged!(Ranged<i32, 0, 10>: value as i32 * factor);",
        ),
        (
            "ensure_ranged",
            "ensure_ranged!(Ranged<i32, 0, 10>: value * factor);",
        ),
    ] {
        let tokens = original(source, name, invocation);
        syn::parse2::<syn::Expr>(tokens).unwrap_or_else(|error| panic!("{invocation}: {error}"));
    }
    let statements = original(source, "cascade", "cascade!(second in 0.. 60 => minute);");
    let block: syn::Block =
        syn::parse2(TokenTree::Group(Group::new(Delimiter::Brace, statements)).into()).unwrap();
    assert_eq!(block.stmts.len(), 3);
    assert!(original(source, "cascade", "cascade!(@ordinal ordinal);").is_empty());
}

#[test]
fn original_repeating_type_families_retain_every_implementation() {
    let count = original(
        include_str!("fixtures/digit_count.rs.txt"),
        "impl_digit_count",
        "impl_digit_count!(u8, u16, u32,);",
    );
    let file: syn::File = syn::parse2(count).unwrap();
    assert_eq!(file.items.len(), 3);
    for item in file.items {
        assert!(matches!(item, syn::Item::Impl(_)));
    }
    let parse = original(
        include_str!("fixtures/shim.rs.txt"),
        "impl_parse_bytes",
        "impl_parse_bytes!(u8 u16 u32 u128);",
    );
    let file: syn::File = syn::parse2(parse).unwrap();
    assert_eq!(file.items.len(), 4);
    for item in file.items {
        assert!(matches!(item, syn::Item::Impl(item) if item.items.len() == 4));
    }
}

#[test]
fn selected_capture_and_template_items_cannot_disappear() {
    let tokens = run("macro_rules! m { ($value:expr) => { fn f() { $value; } impl Drop for Data { fn drop(&mut self) {} } }; }", "m!({ impl Drop for Other { fn drop(&mut self) {} } 0 });").unwrap();
    let file: syn::File = syn::parse2(tokens).unwrap();
    assert_eq!(file.items.len(), 2);
    let syn::Item::Fn(function) = &file.items[0] else {
        panic!("function");
    };
    let syn::Stmt::Expr(syn::Expr::Group(expression), _) = &function.block.stmts[0] else {
        panic!("opaque expr");
    };
    let syn::Expr::Block(block) = expression.expr.as_ref() else {
        panic!("captured block");
    };
    assert!(matches!(
        block.block.stmts.first(),
        Some(syn::Stmt::Item(syn::Item::Impl(_)))
    ));
    assert!(matches!(&file.items[1], syn::Item::Impl(_)));
}

#[test]
fn ordered_arms_select_only_the_first_matching_rule_and_unknown_matchers_fail_closed() {
    let tokens = run("macro_rules! m { (@safe) => { struct Good; }; ($value:tt) => { impl Drop for Data { fn drop(&mut self) {} } }; }", "m!(@safe);").unwrap();
    assert!(matches!(
        syn::parse2::<syn::File>(tokens).unwrap().items[0],
        syn::Item::Struct(_)
    ));
    let tokens = run("macro_rules! m { ($value:tt) => { impl Drop for Data { fn drop(&mut self) {} } }; (safe) => { struct Good; }; }", "m!(safe);").unwrap();
    assert!(matches!(
        syn::parse2::<syn::File>(tokens).unwrap().items[0],
        syn::Item::Impl(_)
    ));
    assert!(
        run(
            "macro_rules! m { ($value:pat) => {}; ($value:tt) => {}; }",
            "m!(safe);"
        )
        .unwrap_err()
        .contains("Unsupported")
    );
}

#[test]
fn typed_forwarding_preserves_opacity_and_exposes_the_actual_later_drop_arm() {
    let outer = run(
        "macro_rules! outer { ($value:expr) => { inner!($value); }; }",
        "outer!(3);",
    )
    .unwrap();
    let invocation: syn::ItemMacro = syn::parse2(outer).unwrap();
    let inner: syn::ItemMacro = syn::parse_str("macro_rules! inner { (3) => { struct Harmless; }; ($value:expr) => { impl Drop for Data { fn drop(&mut self) {} } }; }").unwrap();
    let result = expand(&inner, &invocation.mac, Context { edition: "2024" }).unwrap();
    assert!(matches!(
        syn::parse2::<syn::File>(result).unwrap().items[0],
        syn::Item::Impl(_)
    ));
}

#[test]
fn uncertain_opaque_kind_transitions_are_terminal_and_same_kind_expressions_compose() {
    let outer = run(
        "macro_rules! outer { ($value:expr) => { inner!($value); }; }",
        "outer!(Name);",
    )
    .unwrap();
    let invocation: syn::ItemMacro = syn::parse2(outer).unwrap();
    let inner: syn::ItemMacro = syn::parse_str("macro_rules! inner { ($value:ty) => { struct Harmless; }; ($value:expr) => { impl Drop for Data { fn drop(&mut self) {} } }; }").unwrap();
    assert!(
        expand(&inner, &invocation.mac, Context { edition: "2024" })
            .unwrap_err()
            .contains("opaque fragment kind transition")
    );
    let outer = run(
        "macro_rules! outer { ($value:expr) => { inner!($value + 1); }; }",
        "outer!(value);",
    )
    .unwrap();
    let invocation: syn::ItemMacro = syn::parse2(outer).unwrap();
    let inner: syn::ItemMacro = syn::parse_str(
        "macro_rules! inner { ($value:expr) => { impl Drop for Data { fn drop(&mut self) {} } }; }",
    )
    .unwrap();
    let result = expand(&inner, &invocation.mac, Context { edition: "2024" }).unwrap();
    assert!(matches!(
        syn::parse2::<syn::File>(result).unwrap().items[0],
        syn::Item::Impl(_)
    ));
}

#[test]
fn opaque_literal_to_expression_uncertainty_cannot_select_harmless_fallback() {
    let outer = run(
        "macro_rules! outer { ($value:literal) => { inner!($value); }; }",
        "outer!(3);",
    )
    .unwrap();
    let invocation: syn::ItemMacro = syn::parse2(outer).unwrap();
    let inner: syn::ItemMacro = syn::parse_str("macro_rules! inner { ($value:expr) => { impl Drop for Data { fn drop(&mut self) {} } }; ($value:tt) => { struct Harmless; }; }").unwrap();
    assert!(
        expand(&inner, &invocation.mac, Context { edition: "2024" })
            .unwrap_err()
            .contains("opaque fragment kind transition")
    );
}

#[test]
fn punctuation_uses_rust_compound_tokens_and_ignores_non_glueable_joint_spacing() {
    let list = run(
        "macro_rules! m { ($($name:ident),*) => { $(struct $name;)* }; }",
        "m!(A,B);",
    )
    .unwrap();
    assert_eq!(syn::parse2::<syn::File>(list).unwrap().items.len(), 2);
    for (compound, separated) in [
        ("=>", "= >"),
        ("::", ": :"),
        ("..", ". ."),
        ("..=", ". . ="),
    ] {
        let definition = format!(
            "macro_rules! m {{ ({compound}) => {{ impl Drop for Data {{ fn drop(&mut self) {{}} }} }}; ({separated}) => {{ struct Harmless; }}; }}"
        );
        let tokens = run(&definition, &format!("m!({compound});")).unwrap();
        assert!(matches!(
            syn::parse2::<syn::File>(tokens).unwrap().items[0],
            syn::Item::Impl(_)
        ));
        let tokens = run(&definition, &format!("m!({separated});")).unwrap();
        assert!(matches!(
            syn::parse2::<syn::File>(tokens).unwrap().items[0],
            syn::Item::Struct(_)
        ));
    }
}

#[test]
fn ambiguous_separated_boundaries_and_invalid_expression_followers_are_terminal() {
    let error = run("macro_rules! m { ($($name:ident),* , $last:ident) => {}; ($($any:tt)*) => { struct Harmless; }; }", "m!(A,B);").unwrap_err();
    assert!(error.contains("Ambiguous"));
    let error = run(
        "macro_rules! m { ($value:expr +) => {}; ($($any:tt)*) => { struct Harmless; }; }",
        "m!(1 +);",
    )
    .unwrap_err();
    assert!(error.contains("follow"), "{error}");
}

#[test]
fn expr_edition_changes_and_explicit_2021_fragments_select_the_correct_arm() {
    let definition: syn::ItemMacro = syn::parse_str(
        "macro_rules! m { ($value:expr) => { struct New; }; (const $body:tt) => { struct Old; }; }",
    )
    .unwrap();
    let invocation: syn::ItemMacro = syn::parse_str("m!(const { 1 });").unwrap();
    for (edition, expected) in [("2021", "Old"), ("2024", "New")] {
        let result = expand(&definition, &invocation.mac, Context { edition }).unwrap();
        let syn::Item::Struct(item) = &syn::parse2::<syn::File>(result).unwrap().items[0] else {
            panic!("struct");
        };
        assert_eq!(item.ident, expected);
    }
    assert!(
        run(
            "macro_rules! m { ($value:expr_2021) => {}; }",
            "m!(const { 1 });"
        )
        .is_err()
    );
}

#[test]
fn nested_repetitions_zip_exact_lengths_and_keep_optional_captures() {
    let tokens = run("macro_rules! m { ($( $name:ident : [$($ty:ty),+] );*) => { $(mod $name { $(impl Trait for $ty {})+ })* }; }", "m!(a: [u8,u16]; b: [u32]);").unwrap();
    let file: syn::File = syn::parse2(tokens).unwrap();
    assert_eq!(file.items.len(), 2);
    for (item, expected) in file.items.iter().zip([2, 1]) {
        let syn::Item::Mod(module) = item else {
            panic!("module");
        };
        assert_eq!(module.content.as_ref().unwrap().1.len(), expected);
    }
    assert!(
        run(
            "macro_rules! m { ($($a:ident),* ; $($b:ident),*) => { $(fn $a() { $b(); })* }; }",
            "m!(a,b; c);"
        )
        .unwrap_err()
        .contains("lengths")
    );
}

#[test]
fn rust_type_and_expression_parsers_preserve_nested_commas_and_closures() {
    let tokens = run(
        "macro_rules! m { ($ty:ty, $expr:expr) => { fn f() { let _: $ty = $expr; } }; }",
        "m!(Result<(u8,u16), Error>, |a,b| { let _ = (a,b); 0 });",
    )
    .unwrap();
    assert!(syn::parse2::<syn::File>(tokens).is_ok());
}

#[test]
fn malformed_and_nullable_matchers_or_templates_never_fall_through() {
    for definition in [
        "macro_rules! m {}",
        "macro_rules! m { ($($x:vis)*) => {}; }",
        "macro_rules! m { ($crate:ident) => {}; }",
        "macro_rules! m { ($($x:ident)* $last:ident) => {}; }",
        "macro_rules! m { ($x:ident) => { $missing }; }",
    ] {
        assert!(run(definition, "m!(a);").is_err(), "{definition}");
    }
    let tokens = run("macro_rules! m { () => { $crate::Module }; }", "m!();").unwrap();
    let path: syn::Path = syn::parse2(tokens).unwrap();
    assert_eq!(path.segments[0].ident, "crate");
}

#[test]
fn bounded_lists_are_linear_in_work_and_resource_failures_do_not_select_later_arms() {
    let definition: syn::ItemMacro = syn::parse_str(
        "macro_rules! m { ($($value:ident),*) => { $(struct $value;)* }; ($($any:tt)*) => {}; }",
    )
    .unwrap();
    let mut measures = Vec::new();
    for length in [32, 64] {
        let input = (0..length)
            .map(|index| format!("T{index}"))
            .collect::<Vec<_>>()
            .join(",");
        let rules = grammar::rules(definition.mac.tokens.clone(), &mut Budget::default()).unwrap();
        let mut budget = Budget::default();
        let input = input
            .parse::<TokenStream>()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        assert!(
            matching::complete(
                &rules[0].pattern,
                &input,
                &Context { edition: "2024" },
                &mut budget
            )
            .is_ok()
        );
        measures.push(131_072 - budget.remaining);
    }
    assert!(measures[1] <= measures[0] * 2 + 8);
    let invocation: syn::ItemMacro = syn::parse_str(&format!(
        "m!({});",
        (0..513)
            .map(|index| format!("T{index}"))
            .collect::<Vec<_>>()
            .join(",")
    ))
    .unwrap();
    assert!(
        expand(&definition, &invocation.mac, Context { edition: "2024" })
            .unwrap_err()
            .contains("budget")
    );
}
