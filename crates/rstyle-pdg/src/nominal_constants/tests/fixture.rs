use super::super::*;
use crate::{
    ByteSpan,
    concrete_types::{Field, Nominal, TypeTree},
    data_library::Callee,
};
use syn::spanned::Spanned;

pub(super) struct Fixture {
    pub bytes: String,
    pub syntax: syn::File,
    pub cfgs: Vec<String>,
    pub record: ProductConstant,
    pub deny_authority: bool,
}

impl Fixture {
    pub fn new(offset: &str, expected: &str) -> Self {
        let bytes = source(offset);
        let syntax: syn::File = syn::parse_str(&bytes).unwrap();
        let syn::Item::Impl(implementation) = &syntax.items[1] else {
            panic!()
        };
        let syn::ImplItem::Const(constant) = &implementation.items[0] else {
            panic!()
        };
        let syn::ImplItem::Fn(constructor) = &implementation.items[2] else {
            panic!()
        };
        let record = observation(&bytes, implementation, constant, constructor);
        let record = ProductConstant {
            field: ScalarValue {
                ty: "u32".into(),
                value: expected.into(),
            },
            initializer: Initializer {
                offset: offset.into(),
                ..record.initializer
            },
            ..record
        };

        Self {
            bytes,
            syntax,
            cfgs: vec!["target_pointer_width=\"64\"".into()],
            record,
            deny_authority: false,
        }
    }

    pub fn replace_source(&mut self, from: &str, to: &str) {
        self.bytes = self.bytes.replace(from, to);
        self.syntax = syn::parse_str(&self.bytes).unwrap();
        let syn::Item::Impl(implementation) = &self.syntax.items[1] else {
            panic!()
        };
        let syn::ImplItem::Const(constant) = &implementation.items[0] else {
            panic!()
        };
        let syn::ImplItem::Fn(constructor) = &implementation.items[2] else {
            panic!()
        };
        let observed = observation(&self.bytes, implementation, constant, constructor);
        self.record = ProductConstant {
            field: self.record.field.clone(),
            initializer: Initializer {
                offset: self.record.initializer.offset.clone(),
                ..observed.initializer
            },
            ..observed
        };
    }

    pub fn selection(&self) -> SourceSelection<'_> {
        let syn::Item::Impl(implementation) = &self.syntax.items[1] else {
            panic!()
        };
        let syn::ImplItem::Const(constant) = &implementation.items[0] else {
            panic!()
        };
        let syn::ImplItem::Fn(constructor) = &implementation.items[2] else {
            panic!()
        };
        let unit = || SourceUnit {
            file: "original.rs",
            bytes: &self.bytes,
            implementation,
        };

        SourceSelection {
            constant_unit: unit(),
            constant,
            constructor_unit: unit(),
            constructor,
        }
    }
}

fn source(offset: &str) -> String {
    format!(
        r#"#[derive(Clone, Copy)] struct SmallIndex(u32);
impl SmallIndex {{
    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    pub const MAX: SmallIndex = SmallIndex::new_unchecked(core::i32::MAX as usize - {offset});
    #[cfg(target_pointer_width = "16")]
    pub const MAX: SmallIndex = SmallIndex::new_unchecked(core::isize::MAX - 1);
    #[inline]
    pub const fn new_unchecked(index: usize) -> SmallIndex {{ SmallIndex(index as u32) }}
}}
"#
    )
}

fn observation(
    bytes: &str,
    implementation: &syn::ItemImpl,
    constant: &syn::ImplItemConst,
    method: &syn::ImplItemFn,
) -> ProductConstant {
    let ty = owner();
    ProductConstant {
        identity: IDENTITY.into(),
        definition: local("SmallIndex::MAX"),
        implementation: local("SmallIndex::{impl#0}"),
        declaration: site(bytes, constant.vis.span(), constant.semi_token.span()),
        implementation_source: site(
            bytes,
            implementation.impl_token.span(),
            implementation.brace_token.span.close(),
        ),
        field: ScalarValue {
            ty: "u32".into(),
            value: String::new(),
        },
        initializer: Initializer {
            span: super::super::source::range(bytes, constant.expr.span()),
            core_constant: core(),
            offset: String::new(),
            cast_from: "i32".into(),
            cast_to: "usize".into(),
            subtraction_type: "usize".into(),
        },
        constructor: constructor(bytes, implementation, method, &ty),
        ty,
    }
}

fn constructor(
    bytes: &str,
    implementation: &syn::ItemImpl,
    method: &syn::ImplItemFn,
    ty: &TypeTree,
) -> Constructor {
    let syn::FnArg::Typed(argument) = &method.sig.inputs[0] else {
        panic!()
    };
    Constructor {
        definition: local("SmallIndex::new_unchecked"),
        implementation: local("SmallIndex::{impl#0}"),
        declaration: site(bytes, method.vis.span(), method.block.span()),
        implementation_source: site(
            bytes,
            implementation.impl_token.span(),
            implementation.brace_token.span.close(),
        ),
        parameter_span: super::super::source::range(bytes, argument.pat.span()),
        tuple_constructor: local("SmallIndex::{constructor#0}"),
        parameter_type: "usize".into(),
        result_type: crate::data_types::token(ty).unwrap(),
        field_type: "u32".into(),
    }
}

pub(super) fn owner() -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            package_name: "original".into(),
            package_version: "1.0.0".into(),
            manifest_digest: crate::source_identity::checksum(b"independent manifest"),
            source_identity: Some("original@1.0.0".into()),
            definition_path: "SmallIndex".into(),
            declaration_digest: crate::source_identity::checksum(
                b"#[derive(Clone, Copy)] struct SmallIndex(u32);",
            ),
        },
        source: "#[derive(Clone, Copy)] struct SmallIndex(u32);".into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: "u32".into(),
            ty: TypeTree::Scalar("u32".into()),
        }],
    }
}

pub(super) fn local(path: &str) -> Callee {
    Callee {
        path: path.into(),
        crate_name: "original".into(),
        crate_hash: "local-unencoded".into(),
        definition_hash: format!("independent fixture definition {path}"),
        implementation: None,
    }
}

pub(super) fn core() -> CoreConstant {
    CoreConstant {
        definition: Callee {
            path: "core::legacy_int_modules::i32::MAX".into(),
            crate_name: "core".into(),
            crate_hash: "reviewed-fixture-core".into(),
            definition_hash: "reviewed-fixture-i32-max".into(),
            implementation: None,
        },
        scalar: ScalarValue {
            ty: "i32".into(),
            value: "2147483647".into(),
        },
    }
}

fn site(bytes: &str, start: proc_macro2::Span, end: proc_macro2::Span) -> SourceSite {
    SourceSite {
        file: "original.rs".into(),
        span: ByteSpan {
            start: super::super::source::range(bytes, start).start,
            end: super::super::source::range(bytes, end).end,
        },
    }
}
