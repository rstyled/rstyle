use rstyle_pdg::{HirNode, HirRef, PathRes, ResolvedTy, scalar_constants::ScalarConstant};

fn constant(ty: &str, value: &str) -> ScalarConstant {
    ScalarConstant {
        definition: "sample::Limits::VALUE".into(),
        declaration: format!("pub const VALUE: {ty} = 0;"),
        ty: ty.into(),
        value: value.into(),
    }
}

#[test]
fn scalar_constant_values_respect_signed_ranges_and_target_widths() {
    for (ty, value, width, expected) in [
        ("i8", "-128", None, true),
        ("i8", "128", None, false),
        ("u8", "255", None, true),
        ("u8", "-1", None, false),
        ("u8", "256", None, false),
        (
            "i128",
            "-170141183460469231731687303715884105728",
            None,
            true,
        ),
        (
            "u128",
            "340282366920938463463374607431768211455",
            None,
            true,
        ),
        ("usize", "4294967296", Some(32), false),
        ("usize", "4294967296", Some(64), true),
        ("usize", "1", None, false),
        ("bool", "1", None, false),
        ("bool", "true", None, true),
        ("unit", "()", None, true),
        ("i32", "-0", None, false),
        ("u32", "01", None, false),
        ("f64", "1", None, false),
        ("i08", "1", None, false),
    ] {
        assert_eq!(
            constant(ty, value).value_valid(width),
            expected,
            "{ty} {value}"
        );
    }
}

fn node(constant: &ScalarConstant) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local: 1,
        },
        kind: "expr_path".into(),
        span: rstyle_pdg::span(0, 5),
        snippet: "VALUE".into(),
        ty: Some(ResolvedTy {
            normalized: Some(constant.ty.clone()),
            observed: constant.ty.clone(),
            spelling: String::new(),
        }),
        res: Some(PathRes {
            kind: rstyle_pdg::scalar_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(constant).unwrap(),
        }),
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

#[test]
fn scalar_constant_declarations_and_hir_types_must_agree() {
    let mut constant = constant("i32", "15");
    constant.declaration = "pub const VALUE: i32 = (1 << 4) - 1;".into();
    ScalarConstant::verify(&node(&constant), None).unwrap();
    constant.declaration = "pub const VALUE: u32 = 15;".into();
    assert!(ScalarConstant::verify(&node(&constant), None).is_err());
    constant.declaration = "pub const OTHER: i32 = 15;".into();
    assert!(ScalarConstant::verify(&node(&constant), None).is_err());
    constant.declaration = "pub static VALUE: i32 = 15;".into();
    assert!(ScalarConstant::verify(&node(&constant), None).is_err());
    constant.definition = "sample::type".into();
    constant.declaration = "const r#type: i32 = 15;".into();
    ScalarConstant::verify(&node(&constant), None).unwrap();
}
