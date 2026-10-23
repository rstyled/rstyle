//! A bounded associated-owner compiler lens; observations grant no authority.
use super::*;
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::{GenericArgsRef, Ty};

pub(super) struct Projection<'tcx> {
    owner: LocalDefId,
    arguments: GenericArgsRef<'tcx>,
    pub(super) input: Ty<'tcx>,
    pub(super) span: Ty<'tcx>,
}

impl<'tcx> Projection<'tcx> {
    pub(super) fn new(
        context: &Context<'tcx>,
        owner: LocalDefId,
        argument: &str,
    ) -> Result<Self, String> {
        let tcx = context.tcx;
        check_owner(tcx, owner)?;

        let span = span_type(tcx, context.result)?;
        let input = match argument {
            "Span" => span,
            "Range<usize>" => Ty::new_adt(
                tcx,
                tcx.adt_def(
                    tcx.lang_items()
                        .range_struct()
                        .ok_or("Missing Range lang item")?,
                ),
                tcx.mk_args(&[tcx.types.usize.into()]),
            ),
            _ => return Err("Product instance argument is outside Span/Range<usize>".into()),
        };
        let arguments = tcx.mk_args(&[input.into()]);
        Ok(Self {
            owner,
            arguments,
            input,
            span,
        })
    }

    pub(super) fn arguments(&self) -> GenericArgsRef<'tcx> {
        self.arguments
    }

    pub(super) fn ty(&self, tcx: TyCtxt<'tcx>, original: Ty<'tcx>) -> Result<Ty<'tcx>, String> {
        let value = super::super::projection::normalize(tcx, self.arguments, original)
            .map_err(|error| error.to_string())?;
        types::endpoint(tcx, value)?;
        Ok(value)
    }

    pub(super) fn args(
        &self,
        tcx: TyCtxt<'tcx>,
        original: GenericArgsRef<'tcx>,
    ) -> Result<GenericArgsRef<'tcx>, String> {
        Ok(super::super::projection::complete_arguments(
            tcx,
            super::super::projection::normalize(tcx, self.arguments, original),
        )?
        .arguments)
    }

    pub(super) fn node_ty(
        &self,
        context: &Context<'tcx>,
        id: hir::HirId,
    ) -> Result<Ty<'tcx>, String> {
        super::super::projection::project_node(self.owner, context.typeck, id, |original| {
            self.ty(context.tcx, original)
        })
    }
}

fn check_owner(tcx: TyCtxt<'_>, owner: LocalDefId) -> Result<(), String> {
    let generics = tcx.generics_of(owner);
    let parent = generics
        .parent
        .ok_or("Product constructor lacks its original associated owner")?;
    if tcx.def_kind(owner) != DefKind::AssocFn
        || generics.count() != 1
        || generics.parent_count != 0
        || generics.own_params.len() != 1
        || !matches!(tcx.def_kind(parent), DefKind::Impl { of_trait: false })
        || tcx.generics_of(parent).count() != 0
        || tcx.impl_opt_trait_id(parent).is_some()
        || !matches!(
            generics.own_params[0].kind,
            ty::GenericParamDefKind::Type {
                has_default: false,
                synthetic: false
            }
        )
    {
        return Err("Product lens requires one S0 in an original nongeneric inherent impl".into());
    }
    Ok(())
}

pub(super) fn select<'tcx>(
    tcx: TyCtxt<'tcx>,
    definition: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Result<ty::Instance<'tcx>, Failure> {
    let selected = ty::Instance::try_resolve(
        tcx,
        ty::TypingEnv::fully_monomorphized(),
        definition,
        arguments,
    )
    .map_err(|_| Failure::Error("Product compiler instance selection failed".into()))?
    .ok_or_else(|| {
        Failure::Unresolved("Product compiler instance selection is unresolved".into())
    })?;
    if !matches!(selected.def, ty::InstanceKind::Item(_))
        || !types::arguments(tcx, selected.args).complete()
    {
        return Err("Product selected instance is non-Item or structurally incomplete".into());
    }
    Ok(selected)
}

fn span_type<'tcx>(tcx: TyCtxt<'tcx>, result: Ty<'tcx>) -> Result<Ty<'tcx>, String> {
    let ty::Adt(product, arguments) = result.kind() else {
        return Err("Product result is not an ADT".into());
    };
    if !product.is_struct() {
        return Err("Product result is not a struct".into());
    }
    let fields = &product.non_enum_variant().fields;
    if !fields
        .iter()
        .map(|field| field.name.as_str())
        .eq(["pattern", "span"])
        || !product.did().is_local()
        || tcx.item_name(product.did()).as_str() != "Match"
    {
        return Err("Product lens requires the actual complete Match result".into());
    }

    let field = fields.iter().nth(1).ok_or("Missing Match.span field")?;
    let span = field_type(tcx, field, arguments)?;
    let ty::Adt(owner, arguments) = span.kind() else {
        return Err("Match.span is not an ADT".into());
    };
    if !owner.is_struct() {
        return Err("Match.span is not a struct".into());
    }
    let fields = &owner.non_enum_variant().fields;
    if !owner.did().is_local()
        || !fields
            .iter()
            .map(|field| field.name.as_str())
            .eq(["start", "end"])
    {
        return Err("Product lens requires actual local Span with two ordered usize fields".into());
    }
    for field in fields.iter() {
        if field_type(tcx, field, arguments)? != tcx.types.usize {
            return Err("Product Span field is not concrete usize".into());
        }
    }
    Ok(span)
}

fn field_type<'tcx>(
    tcx: TyCtxt<'tcx>,
    field: &ty::FieldDef,
    arguments: GenericArgsRef<'tcx>,
) -> Result<Ty<'tcx>, String> {
    tcx.try_normalize_erasing_regions(
        ty::TypingEnv::fully_monomorphized(),
        field.ty(tcx, arguments),
    )
    .map_err(|_| "Product field type normalization failed".into())
}

#[derive(Serialize)]
pub(super) struct Owner {
    definition: CompilerDefinition,
    parent: Option<CompilerDefinition>,
    declaration: Anchor,
    parameters: Vec<Parameter>,
    source_predicates: Vec<Anchor>,
    clauses: Vec<Clause>,
}

#[derive(Serialize)]
struct Parameter {
    definition: CompilerDefinition,
    index: u32,
    name: String,
    declaration: Anchor,
    name_anchor: Anchor,
}

#[derive(Serialize)]
struct Clause {
    anchor: Option<Anchor>,
    trait_definition: CompilerDefinition,
    positive: bool,
    original_arguments: Vec<OriginalType>,
    substituted_arguments: types::Arguments,
}

#[derive(Serialize)]
pub(super) struct Anchor {
    source_file: String,
    span: rstyle_pdg::ByteSpan,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum OriginalType {
    Parameter {
        index: u32,
        name: String,
    },
    Adt {
        definition: CompilerDefinition,
        arguments: Vec<OriginalType>,
    },
    Builtin {
        name: String,
    },
    Str,
    Reference {
        region: String,
        mutable: bool,
        referent: Box<OriginalType>,
    },
    Tuple {
        elements: Vec<OriginalType>,
    },
}

pub(super) fn owner<'tcx>(
    context: &Context<'tcx>,
    id: LocalDefId,
    lens: &Projection<'tcx>,
) -> Result<Owner, String> {
    let tcx = context.tcx;
    let generics = tcx.generics_of(id);
    let source = tcx
        .hir_get_generics(id)
        .ok_or("Product owner lacks original generic source")?;
    let clauses = tcx.clauses_of(id);
    if clauses.parent != generics.parent
        || clauses.clauses.len() > 16
        || source.params.len() != generics.own_params.len()
    {
        return Err("Product owner source/compiler parameter or clause rosters disagree".into());
    }

    let parameters = source
        .params
        .iter()
        .zip(&generics.own_params)
        .map(|(source, compiler)| parameter(context, source, compiler))
        .collect::<Result<_, String>>()?;
    Ok(Owner {
        definition: compiler_definition(tcx, id.to_def_id()),
        parent: generics.parent.map(|id| compiler_definition(tcx, id)),
        declaration: anchor(context, tcx.def_span(id))?,
        parameters,
        source_predicates: source
            .predicates
            .iter()
            .map(|predicate| anchor(context, predicate.span))
            .collect::<Result<_, _>>()?,
        clauses: clauses
            .clauses
            .iter()
            .map(|(value, span)| clause(context, *value, *span, lens))
            .collect::<Result<_, _>>()?,
    })
}

fn parameter(
    context: &Context<'_>,
    source: &hir::GenericParam<'_>,
    compiler: &ty::GenericParamDef,
) -> Result<Parameter, String> {
    if source.def_id.to_def_id() != compiler.def_id
        || source.pure_wrt_drop
        || compiler.pure_wrt_drop
        || !matches!(
            compiler.kind,
            ty::GenericParamDefKind::Type {
                has_default: false,
                synthetic: false
            }
        )
    {
        return Err("Product source/compiler generic parameter disagrees".into());
    }

    Ok(Parameter {
        definition: compiler_definition(context.tcx, compiler.def_id),
        index: compiler.index,
        name: compiler.name.to_string(),
        declaration: anchor(context, source.span)?,
        name_anchor: anchor(context, source.name.ident().span)?,
    })
}

fn clause<'tcx>(
    context: &Context<'tcx>,
    value: ty::Clause<'tcx>,
    span: rustc_span::Span,
    lens: &Projection<'tcx>,
) -> Result<Clause, String> {
    let bound = value.kind();
    if !bound.bound_vars().is_empty() {
        return Err("Higher-ranked product clause unsupported".into());
    }
    let ty::ClauseKind::Trait(predicate) = bound.skip_binder() else {
        return Err("Product owner has a non-trait clause outside the bounded lens".into());
    };
    Ok(Clause {
        anchor: if span.is_dummy() {
            None
        } else {
            Some(anchor(context, span)?)
        },
        trait_definition: compiler_definition(context.tcx, predicate.trait_ref.def_id),
        positive: predicate.polarity == ty::ClausePolarity::Positive,
        original_arguments: predicate
            .trait_ref
            .args
            .iter()
            .map(|argument| match argument.kind() {
                ty::GenericArgKind::Type(value) => original_type(context.tcx, value, 0),
                _ => Err("Product clause has non-type arguments".into()),
            })
            .collect::<Result<_, _>>()?,
        substituted_arguments: types::arguments(
            context.tcx,
            lens.args(context.tcx, predicate.trait_ref.args)?,
        ),
    })
}

pub(super) fn original_type<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: Ty<'tcx>,
    depth: usize,
) -> Result<OriginalType, String> {
    if depth > 16 {
        return Err("Product symbolic type exceeds depth budget".into());
    }
    Ok(match value.kind() {
        ty::Param(parameter) => OriginalType::Parameter {
            index: parameter.index,
            name: parameter.name.to_string(),
        },
        ty::Bool | ty::Uint(_) | ty::Int(_) | ty::Never => OriginalType::Builtin {
            name: value.to_string(),
        },
        ty::Str => OriginalType::Str,
        ty::Ref(region, referent, mutable) => OriginalType::Reference {
            region: format!("{region:?}"),
            mutable: mutable.is_mut(),
            referent: Box::new(original_type(tcx, *referent, depth + 1)?),
        },
        ty::Tuple(elements) => OriginalType::Tuple {
            elements: elements
                .iter()
                .map(|value| original_type(tcx, value, depth + 1))
                .collect::<Result<_, _>>()?,
        },
        ty::Adt(definition, arguments) => OriginalType::Adt {
            definition: compiler_definition(tcx, definition.did()),
            arguments: arguments
                .iter()
                .map(|argument| match argument.kind() {
                    ty::GenericArgKind::Type(value) => original_type(tcx, value, depth + 1),
                    _ => Err("Product symbolic ADT has non-type arguments".into()),
                })
                .collect::<Result<_, _>>()?,
        },
        _ => return Err("Original product type is outside the bounded symbolic grammar".into()),
    })
}

fn anchor(context: &Context<'_>, span: rustc_span::Span) -> Result<Anchor, String> {
    if span.from_expansion() || span.is_dummy() {
        return Err("Product owner anchor is expanded or absent".into());
    }
    let file = context.tcx.sess.source_map().lookup_source_file(span.lo());
    let rustc_span::FileName::Real(real) = &file.name else {
        return Err("Product source has no original filesystem path".into());
    };
    let path = real
        .local_path()
        .ok_or("Product source lacks a local path")?;
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };

    let (_, span) = context
        .sources
        .range(context.tcx, span)
        .ok_or("Product owner anchor crosses source files")?;
    Ok(Anchor {
        source_file: path.to_string_lossy().into_owned(),
        span,
    })
}
