use super::*;

pub(super) fn verify(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    cfg: &flow::Flow<'_>,
    replay: &Replay<'_>,
) -> Result<(), String> {
    let last = replay
        .selectors
        .last()
        .ok_or("Missing final prefix selector")?;
    for (index, digit) in checked.digits().iter().enumerate() {
        element(pair, cfg, replay, (index, digit))?;
    }
    let suffix = replay.suffix.0;
    let declaration = binding(pair, checked.prefix().remainder().binding(), suffix)?;
    verify_saved_operands(pair, suffix, &[replay.saved])?;
    cfg.outgoing(suffix, &[(CfgEdgeKind::Normal, declaration)])?;
    cfg.guarded(last, suffix, CfgEdgeKind::True)?;
    cfg.guarded(last, declaration, CfgEdgeKind::True)?;
    let remainder = value_operation(pair, checked.remainder())?;
    reaching(pair, declaration, remainder)?;
    cfg.before(declaration, remainder)?;

    success(pair, checked, cfg, (last, declaration))
}

fn element(
    pair: PairView<'_>,
    cfg: &flow::Flow<'_>,
    replay: &Replay<'_>,
    digit: (usize, &super::super::semantics::Digit<'_>),
) -> Result<(), String> {
    let (index, digit) = digit;
    let last = replay
        .selectors
        .last()
        .ok_or("Missing final prefix selector")?;
    let element = replay.elements[index].0;
    let declaration = binding(pair, digit.binding(), element)?;
    let pointer = value_operation(pair, digit.pointer())?;
    let read = value_operation(pair, digit.read())?;
    verify_saved_operands(pair, element, &[replay.saved])?;
    verify_saved_operands(pair, read, &[pointer])?;
    reaching(pair, declaration, pointer)?;
    if replay
        .selector_reads
        .iter()
        .any(|(selector, _)| selector.id == read.id)
    {
        return Err("Prefix selector read cannot replace original body dereference".into());
    }
    cfg.outgoing(element, &[(CfgEdgeKind::Normal, declaration)])?;
    cfg.guarded(last, element, CfgEdgeKind::True)?;
    cfg.guarded(last, declaration, CfgEdgeKind::True)?;
    cfg.before(declaration, pointer)?;
    cfg.before(pointer, read)?;
    let next = replay
        .elements
        .get(index + 1)
        .map_or(replay.suffix.0, |(node, _)| *node);
    cfg.outgoing(declaration, &[(CfgEdgeKind::Normal, next)])?;

    Ok(())
}

fn success(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    cfg: &flow::Flow<'_>,
    guards: (&PdgNode, &PdgNode),
) -> Result<(), String> {
    let body = checked.success();
    for node in &pair.pdg.nodes {
        if node.category != NodeCategory::Merge
            && node.source_span.as_ref().is_some_and(|span| {
                span.start >= u64::from(body.span.start) && span.end <= u64::from(body.span.end)
            })
        {
            cfg.guarded(guards.0, node, CfgEdgeKind::True)?;
            cfg.before(guards.1, node)?;
        }
    }
    Ok(())
}

fn binding<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    value: &PdgNode,
) -> Result<&'a PdgNode, String> {
    let node = serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Declaration
    })?;
    operations::role(node, "prefix_binding")?;
    crate::data_documents::verify_binding_initialization(node, source)?;
    verify_saved_operands(pair, node, &[value])?;

    Ok(node)
}

fn reaching(pair: PairView<'_>, binding: &PdgNode, pointer: &PdgNode) -> Result<(), String> {
    crate::data_documents::verify_single_data_source(pair, pointer, binding, "reaching_definition")
}
