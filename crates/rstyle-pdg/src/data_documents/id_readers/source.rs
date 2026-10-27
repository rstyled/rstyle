//! Complete ID reader source: checked bytes, saved decoder and exact error flow.
use super::*;
use crate::{data_types::TypeToken, named_error_result_propagation as propagation};

pub(super) struct Checked<'a> {
    pub bytes_local: &'a HirNode,
    pub bytes_binding: &'a HirNode,
    pub unwrap: &'a HirNode,
    pub copy: &'a HirNode,
    pub view: crate::data_subslices::Checked<'a>,
    pub id_local: &'a HirNode,
    pub id_binding: &'a HirNode,
    pub question: &'a HirNode,
    pub propagation: propagation::Checked<'a>,
    pub map: crate::id_error_maps::Checked<'a>,
    pub decoder: &'a HirNode,
    pub bytes_read: &'a HirNode,
    pub success: &'a HirNode,
    pub tuple: &'a HirNode,
    pub id_read: &'a HirNode,
    pub size: &'a HirNode,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    syntax_interface(&syntax)?;
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    interface(facts)?;
    super::super::wire_validators::source::statement_order(facts, &syntax)?;

    let [bytes_local, id_local, success] = facts.body.children.as_slice() else {
        return Err("ID reader requires its complete two declarations and success tail".into());
    };
    let bytes = bytes(bytes_local, facts)?;
    let decoded = decode(id_local, &bytes, facts)?;
    let tail = tail(success, decoded.binding, facts)?;
    Ok(Some(Checked {
        bytes_local,
        bytes_binding: bytes.binding,
        unwrap: bytes.unwrap,
        copy: bytes.copy,
        view: bytes.view,
        id_local,
        id_binding: decoded.binding,
        question: decoded.question,
        propagation: decoded.propagation,
        map: decoded.map,
        decoder: decoded.decoder,
        bytes_read: decoded.bytes_read,
        success,
        tuple: tail.tuple,
        id_read: tail.id_read,
        size: tail.size,
    }))
}

struct Bytes<'a> {
    binding: &'a HirNode,
    unwrap: &'a HirNode,
    copy: &'a HirNode,
    view: crate::data_subslices::Checked<'a>,
}

fn bytes<'a>(bytes_local: &'a HirNode, facts: &'a dyn EvidenceFacts) -> Result<Bytes<'a>, String> {
    let [bytes_binding, annotation, unwrap] = bytes_local.children.as_slice() else {
        return Err("ID reader requires its original typed byte-array binding".into());
    };
    immutable(bytes_local)?;
    if annotation.kind != "ty_annotation"
        || crate::data_bytes::verify(unwrap, facts)? != crate::data_bytes::Kind::Unwrap(4)
    {
        return Err("ID reader requires its exact four-byte array unwrap".into());
    }
    let [copy] = unwrap.children.as_slice() else {
        return Err("ID reader unwrap must consume one saved conversion".into());
    };
    if crate::data_bytes::verify(copy, facts)? != crate::data_bytes::Kind::Copy(4) {
        return Err("ID reader changes its exact four-byte copy".into());
    }
    let [window] = copy.children.as_slice() else {
        return Err("ID reader copy requires the sole original prefix".into());
    };
    let view = crate::data_subslices::verify(window, facts)?;
    if view.start.is_some() || view.array_length.is_some() {
        return Err("ID reader requires a slice prefix".into());
    }
    formal(view.base, 0, facts)?;
    let end = view.end.ok_or("Missing ID byte prefix endpoint")?;
    size(end, facts, WidthSite::Expression)?;
    size(annotation, facts, WidthSite::ArrayAnnotation)?;

    Ok(Bytes {
        binding: bytes_binding,
        unwrap,
        copy,
        view,
    })
}

struct Decoded<'a> {
    binding: &'a HirNode,
    question: &'a HirNode,
    propagation: propagation::Checked<'a>,
    map: crate::id_error_maps::Checked<'a>,
    decoder: &'a HirNode,
    bytes_read: &'a HirNode,
}

fn decode<'a>(
    id_local: &'a HirNode,
    bytes: &Bytes<'a>,
    facts: &'a dyn EvidenceFacts,
) -> Result<Decoded<'a>, String> {
    let [id_binding, question] = id_local.children.as_slice() else {
        return Err("ID reader must save exactly the propagated decoder result".into());
    };
    immutable(id_local)?;
    let propagation = propagation::verify(question, facts)?;
    if propagation.mode != propagation::Mode::IdToReader {
        return Err("ID reader changes its nominal-to-reader continuation".into());
    }
    let map = crate::id_error_maps::verify(propagation.operand, facts)?;
    let decoder = map.receiver;
    let [bytes_read] = decoder.children.as_slice() else {
        return Err("ID decoder requires one saved byte array".into());
    };
    read(bytes_read, bytes.binding)?;
    let helper = crate::helper_calls::verify_use(decoder, facts)?;
    let summary = helper.summary();
    if summary.identity != "rstyle.helper.source-body/1"
        || summary.may_panic
        || !summary.always_returns
        || summary.memory_effects
        || summary.mutates_arguments
        || summary.argument_types != [crate::data_aggregates::token(bytes_read)?]
        || summary.result_type != crate::data_aggregates::token(decoder)?
    {
        return Err("ID reader requires its complete verified checked decoder".into());
    }
    if crate::data_aggregates::parsed_type(id_binding)?
        != crate::data_aggregates::parsed_type(question)?
    {
        return Err("ID reader binding changes its exact saved nominal payload".into());
    }

    Ok(Decoded {
        binding: id_binding,
        question,
        propagation,
        map,
        decoder,
        bytes_read,
    })
}

struct Tail<'a> {
    tuple: &'a HirNode,
    id_read: &'a HirNode,
    size: &'a HirNode,
}

fn tail<'a>(
    success: &'a HirNode,
    id_binding: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Tail<'a>, String> {
    let constructed = crate::concrete_results::verify_constructor(success, facts)?;
    let tuple = constructed.payload;
    let [id_read, size_node] = tuple.children.as_slice() else {
        return Err("ID reader must return both original success components".into());
    };
    if constructed.construct != "data.result-ok" || tuple.kind != "expr_tup" {
        return Err("ID reader requires the exact successful ID/count tuple".into());
    }
    read(id_read, id_binding)?;
    size(size_node, facts, WidthSite::Expression)?;

    Ok(Tail {
        tuple,
        id_read,
        size: size_node,
    })
}

pub(super) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::source_evidence::initial_statement(facts, CAPABILITY, |first| {
        first.kind == "stmt_let"
            && facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .is_some_and(|text| {
                    matches!(crate::data_types::parse(text), Ok(TypeToken::Result { ok, .. })
                    if crate::id_reader_types::shape(&ok))
                })
    })
}

fn syntax_interface(syntax: &syn::ItemFn) -> Result<(), String> {
    super::super::wire_validators::source::syntax_interface(syntax)?;
    if !matches!(
        syntax.block.stmts.as_slice(),
        [
            syn::Stmt::Local(_),
            syn::Stmt::Local(_),
            syn::Stmt::Expr(syn::Expr::Call(_), None)
        ]
    ) {
        return Err("ID reader source requires two complete bindings and one success tail".into());
    }
    Ok(())
}

fn interface(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let signature = &facts.signature;
    if signature.params.len() != 2
        || !super::super::fallible_readers::source::implicit_slice_lifetime(facts)
        || signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_unsafe
        || signature.is_const
        || signature.is_variadic
        || signature.abi.value.as_deref() != Some("Rust")
        || facts.body.kind != "expr_block"
        || facts.body.expansion.is_some()
        || facts.body.is_unsafe_block
    {
        return Err("ID reader requires its complete ordinary two-formal interface".into());
    }
    let slice = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));
    if crate::data_aggregates::parsed_type(&signature.params[0].pat)? != slice {
        return Err("ID reader requires shared input bytes".into());
    }
    crate::named_error_payloads::static_formal(facts, 1)?;

    Ok(())
}

fn immutable(local: &HirNode) -> Result<(), String> {
    let Some(binding) = local
        .children
        .first()
        .filter(|_| local.kind == "stmt_let" && local.expansion.is_none())
    else {
        return Err("ID reader requires an original saved declaration".into());
    };
    crate::source_evidence::plain_immutable_binding(
        binding,
        "ID reader requires plain immutable saved bindings",
    )
}

fn read(node: &HirNode, binding: &HirNode) -> Result<(), String> {
    crate::source_evidence::verify_binding_read(
        node,
        binding,
        "ID reader changes its saved binding",
    )
}
fn formal(node: &HirNode, position: usize, facts: &dyn EvidenceFacts) -> Result<(), String> {
    super::super::fallible_readers::source::formal(node, position, facts)
}
enum WidthSite {
    Expression,
    ArrayAnnotation,
}

fn size(node: &HirNode, facts: &dyn EvidenceFacts, site: WidthSite) -> Result<(), String> {
    let resolution = facts.nominal_resolution()?;
    let width = match site {
        WidthSite::Expression => resolution.verify_id_reader_constant(node, facts)?,
        WidthSite::ArrayAnnotation => resolution.verify_id_reader_annotation(node, facts)?,
    };
    if width != 4 {
        return Err("ID reader requires its independently proved four-byte width".into());
    }
    Ok(())
}
