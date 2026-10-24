//! Raw generated tuple declaration evidence; substitution and cleanup are separate.
use super::*;
use rstyle_pdg::{SourceSnapshot, SourceText, declaration_origins::GeneratedOrigin};
use rustc_hir::{ItemKind, VariantData};

pub(super) fn capture<'tcx>(
    capture: &mut Capture<'_, 'tcx>,
    instance: Instance<'tcx>,
    depth: usize,
) -> Result<Option<TypeTree>, Failure> {
    let id = instance.definition.did();
    let Node::Item(item) = capture.ctx.tcx.hir_node_by_def_id(id.expect_local()) else {
        return Err(unsupported("Missing concrete struct HIR item"));
    };
    if !item.span.from_expansion() {
        return Ok(None);
    }
    if !capture
        .ctx
        .profile
        .supports(rstyle_pdg::result_maps::CAPABILITY)
    {
        return Err(unsupported(
            "Generated declaration requires its registered Result-map capability",
        ));
    }

    let raw = Raw::capture(capture.ctx, id, item)?;
    let fields =
        capture.struct_fields(instance, vec![("0".into(), raw.field.to_string())], depth)?;
    let nominal = nominal(capture.ctx, capture.ctx.tcx.def_path_str(id), &raw.template)?;
    let definition = raw.definition.clone();
    let template = raw.template.clone();
    let fields = exact_field(fields, raw.field.clone())?;

    capture
        .generated_origins
        .insert(nominal.definition_path.clone(), raw.bind(nominal.clone()));
    Ok(Some(TypeTree::GeneratedStruct {
        nominal,
        template,
        definition: Box::new(definition),
        fields,
    }))
}

fn exact_field(mut fields: Vec<Field>, declaration: SourceText) -> Result<Vec<Field>, Failure> {
    let [field] = fields.as_mut_slice() else {
        return Err(unsupported("Generated tuple field inventory disagrees"));
    };
    field.declaration = declaration;
    Ok(fields)
}

struct Raw {
    template: SourceText,
    field: SourceText,
    definition: rstyle_pdg::generated_sources::capture::Definition,
    origin: rstyle_pdg::generated_sources::capture::Origin,
    defining_expansion: String,
    file_bytes: SourceSnapshot,
}

impl Raw {
    fn capture(ctx: &Context<'_>, id: DefId, item: &rustc_hir::Item<'_>) -> Result<Self, Failure> {
        let ItemKind::Struct(_, generics, VariantData::Tuple(fields, ..)) = item.kind else {
            return Err(unsupported(
                "Generated declaration requires a unary tuple struct",
            ));
        };
        if !generics.params.is_empty() || !generics.predicates.is_empty() || fields.len() != 1 {
            return Err(unsupported(
                "Generated tuple declaration has generics or another field inventory",
            ));
        }

        let template = original(ctx, item.span)?;
        let field = original(ctx, fields[0].ty.span)?;
        rstyle_pdg::declaration_origins::generated_field_type(&field)
            .map_err(Failure::Unsupported)?;

        let (definition, origin, defining_expansion) =
            super::super::generated_sources::declaration_origin(ctx, id, item.span)
                .map_err(Failure::Unsupported)?;
        let file = ctx.tcx.sess.source_map().lookup_source_file(item.span.lo());
        let snapshot = ctx
            .sources
            .file(&file)
            .ok_or_else(|| unsupported("Missing generated declaration original file"))?;
        Ok(Self {
            template,
            field,
            definition,
            origin,
            defining_expansion,
            file_bytes: snapshot.bytes.clone(),
        })
    }

    fn bind(self, nominal: Nominal) -> GeneratedOrigin {
        GeneratedOrigin {
            nominal,
            definition: self.definition,
            origin: self.origin,
            defining_expansion: self.defining_expansion,
            file_bytes: self.file_bytes,
        }
    }
}

fn original(ctx: &Context<'_>, span: rustc_span::Span) -> Result<SourceText, Failure> {
    let (source, _) = ctx
        .sources
        .range(ctx.tcx, span)
        .ok_or_else(|| unsupported("Missing exact raw generated declaration bytes"))?;
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Failure::Budget(
            "Generated declaration exceeds source budget".into(),
        ));
    }
    Ok(source)
}
