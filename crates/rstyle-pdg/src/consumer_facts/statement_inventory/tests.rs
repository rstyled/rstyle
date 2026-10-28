use super::*;
use crate::consumer_facts::tests::{facts, source_owner};

#[test]
fn explicit_source_counts_include_all_blocks_tails_and_bare_match_arms() {
    for (source, expected) in [
        ("fn empty() {}", 0),
        ("fn tail() { 42 }", 1),
        ("fn local() { let x = { first(); 1 }; x }", 4),
        (
            "fn branches() { if true { first(); 1 } else { 2 }; last(); }",
            5,
        ),
        (
            "fn loops() { while false { first(); } for x in 0..0 { consume(x); } }",
            4,
        ),
        (
            "fn choices() { match 0 { 0 => first(), _ => { second(); 2 } } }",
            4,
        ),
        ("fn unreachable() { return; first(); { second(); } }", 4),
        ("fn expressions() { consume(first() + second()); }", 1),
        ("fn café() {\r\n let π = 1;\r\n { π }\r\n}", 3),
    ] {
        assert_eq!(
            inspect(&facts(source)).unwrap(),
            Inventory {
                statements: Availability::Known(expected),
                execution: InspectionExecution::Completed,
            },
            "{source}"
        );
    }
}

#[test]
fn opaque_boundaries_never_publish_a_supported_remainder() {
    for source in [
        "fn nested() { first(); fn inner() { second(); } last(); }",
        "fn closure() { first(); let x = || { second(); }; last(); }",
        "fn macro_body() { first(); expand!(); last(); }",
        "fn macro_value() { let x = expand!(); last(); }",
        "fn unsafe_block() { first(); unsafe { second(); } }",
        "unsafe fn unsafe_function() { first(); }",
        "async fn suspended() { first(); }",
        "fn async_block() { let x = async { first(); }; }",
        "fn constant() { let x = const { 1 }; }",
        "#[allow(unused)] fn attributed() { first(); }",
        "fn conditional() { #[cfg(unix)] let x = 1; last(); }",
        "fn arm() { match 0 { #[cfg(unix)] 0 => first(), _ => last() } }",
        "fn nested_attribute() { let x = { #[allow(unused)] first(); 1 }; }",
        "fn empty_statement() { ; }",
    ] {
        let mut captured = facts(source);
        let function = syn::parse_str(source).unwrap();
        captured.signature.sig_span = super::super::header_span(source, &function);
        assert!(
            matches!(
                inspect(&captured).unwrap().statements,
                Availability::Unsupported { .. }
            ),
            "{source}"
        );
    }
}

#[test]
fn compile_time_blocks_never_enlarge_the_source_denominator() {
    for source in [
        "fn array_type() { let a: [u32; { 1 }] = [0]; use_it(a); }",
        "fn array_repeat() { let a = [0; { 1 }]; use_it(a); }",
        "fn const_argument() { invoke::<{ { 1 } }>(); }",
        "fn associated_const() { invoke::<Trait<VALUE = { 1 }>>(); }",
    ] {
        assert!(
            matches!(
                inspect(&facts(source)).unwrap().statements,
                Availability::Unsupported { .. }
            ),
            "{source}"
        );
    }
}

#[test]
fn ownership_and_signature_contradictions_reject_even_after_resealing() {
    let mut changed = facts("fn original() { first(); }");
    changed.source.bytes = "fn original() { other(); }".into();
    changed.source.digest = changed.source.bytes.checksum().into();
    assert!(inspect(&changed).is_err());

    let mut signature = facts("fn original() { first(); }");
    signature.signature.sig_span.end -= 1;
    assert!(inspect(&signature).is_err());

    let mut no_owner = facts("fn original() { first(); }");
    no_owner.build.frontend_options = Default::default();
    assert!(matches!(
        inspect(&no_owner).unwrap().statements,
        Availability::Unknown { .. }
    ));
}

#[test]
fn excessive_source_is_unavailable_before_body_parsing() {
    let mut captured = facts("fn original() { 1 }");
    let source = format!(
        "fn original() {{ {}1{} }}",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    captured.source.bytes = source.as_str().into();
    captured.source.digest = captured.source.bytes.checksum().into();
    captured.fn_span.end = source.len() as u32;
    captured.body.span.end = source.len() as u32;
    captured.body.snippet = source[captured.body.span.start as usize..].into();
    source_owner(&mut captured, &source);
    let inventory = inspect(&captured).unwrap();
    assert_eq!(inventory.execution, InspectionExecution::BudgetExhausted);
    assert!(
        matches!(inventory.statements, Availability::Unknown { reason } if reason.contains("budget"))
    );
}
