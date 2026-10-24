//! Output accounting follows physical tokens, independent of template nesting.
use super::*;

fn count(tokens: TokenStream) -> usize {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => 1 + count(group.stream()),
            _ => 1,
        })
        .sum()
}

#[test]
fn nested_repetition_and_opaque_captures_count_each_emitted_token_once() {
    let definition: syn::ItemMacro = syn::parse_str(
        "macro_rules! m { ($($t:ty),*) => { mod out { $(fn f() { let _: $t; })* } }; }",
    )
    .unwrap();
    let mut budget = Budget::default();
    let rules = grammar::rules(definition.mac.tokens, &mut budget).unwrap();
    let input = "u8, (u16, u32), [u8; 4]".parse::<TokenStream>().unwrap();
    let input = input.into_iter().collect::<Vec<_>>();
    let captures = matching::complete(
        &rules[0].pattern,
        &input,
        &Context { edition: "2024" },
        &mut budget,
    )
    .unwrap_or_else(|_| panic!("complete valid captures"));
    let output = expand(&rules[0].body, &captures, &mut budget).unwrap();
    assert_eq!(budget.emitted, count(output.clone()));
    let file = syn::parse2::<syn::File>(output).unwrap();
    let syn::Item::Mod(module) = &file.items[0] else {
        panic!("complete module");
    };
    assert_eq!(module.content.as_ref().unwrap().1.len(), 3);
}

#[test]
fn actual_output_limit_includes_captured_tokens_groups_and_separators() {
    let token: TokenTree = "x"
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut budget = Budget::default();
    budget
        .emit(std::iter::repeat_n(token.clone(), 32_767).collect())
        .unwrap();
    budget.emit(token.clone().into()).unwrap();
    assert_eq!(budget.emitted, 32_768);
    assert!(
        budget
            .emit(token.into())
            .unwrap_err()
            .contains("output budget")
    );
}

#[test]
fn repeated_separators_contribute_to_the_physical_output_count() {
    let tokens = "x".parse::<TokenStream>().unwrap();
    let captures = Captures::from([(
        "x".into(),
        Value::Many(vec![Value::Atom(tokens.clone()), Value::Atom(tokens)]),
    )]);
    let comma = ","
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let body = [Node::Repeat {
        body: vec![Node::Capture("x".into(), None)],
        separator: Some(comma),
        operator: '*',
    }];

    let mut budget = Budget::default();
    let output = expand(&body, &captures, &mut budget).unwrap();
    assert_eq!(output.to_string(), "x , x");
    assert_eq!(budget.emitted, 3);
    assert_eq!(count(output), 3);
}

#[test]
fn original_component_macro_keeps_all_seventeen_variants_and_method_owners() {
    let file = syn::parse_file(include_str!("../fixtures/component_definition.rs.txt")).unwrap();
    let macros = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Macro(item) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    let tokens =
        super::super::expand(macros[0], &macros[1].mac, Context { edition: "2024" }).unwrap();
    let expanded = syn::parse2::<syn::File>(tokens).unwrap();
    let syn::Item::Enum(owner) = &expanded.items[0] else {
        panic!("complete component enum");
    };
    assert_eq!(owner.variants.len(), 17);
    assert_eq!(
        expanded
            .items
            .iter()
            .filter(|item| matches!(item, syn::Item::Struct(_)))
            .count(),
        17
    );
    assert_eq!(
        expanded
            .items
            .iter()
            .filter(|item| matches!(item, syn::Item::Impl(_)))
            .count(),
        18
    );
    assert_eq!(
        expanded
            .items
            .iter()
            .filter(|item| matches!(item, syn::Item::Fn(_)))
            .count(),
        1
    );
    assert_eq!(expanded.items.len(), 37);
}

#[test]
fn combined_capture_and_template_depth_remains_bounded() {
    let definition: syn::ItemMacro =
        syn::parse_str("macro_rules! m { ($t:tt) => { mod out { fn f() { $t; } } }; }").unwrap();
    let mut tokens = "x".parse::<TokenStream>().unwrap();
    for _ in 0..48 {
        tokens = TokenTree::Group(Group::new(Delimiter::Parenthesis, tokens)).into();
    }
    let invocation = syn::Macro {
        path: syn::parse_str("m").unwrap(),
        bang_token: Default::default(),
        delimiter: syn::MacroDelimiter::Paren(Default::default()),
        tokens,
    };
    assert!(
        super::super::expand(&definition, &invocation, Context { edition: "2024" })
            .unwrap_err()
            .contains("nesting budget")
    );
}

#[test]
fn nested_output_work_grows_with_tokens_instead_of_rewalking_every_parent() {
    let token = "x"
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut work = Vec::new();
    for depth in [4, 32] {
        let mut body = vec![Node::Token(token.clone()); 128];
        for _ in 0..depth {
            body = vec![Node::Group(Delimiter::Brace, body)];
        }
        let mut budget = Budget::default();
        let output = expand(&body, &Captures::new(), &mut budget).unwrap();
        assert_eq!(budget.emitted, 128 + depth);
        assert_eq!(count(output), 128 + depth);
        work.push(131_072 - budget.remaining);
    }
    assert!(work[1] < work[0] * 2, "work at depth 4/32: {work:?}");
}
