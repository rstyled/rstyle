//! Observational type capture for the unavailable data tier. No eligibility flag.

mod callbacks;
mod copy;
mod fieldless_enums;
mod generated_declarations;
mod mixed_enums;
mod nonzero_values;
mod range_values;
mod receivers;
mod reuse;
mod scalar_enums;
mod scalar_product_instances;

use super::hir::Context;
use rstyle_pdg::{HirFnFacts, concrete_types::*};
use rustc_hir::{Mutability, Node, def_id::DefId};
use rustc_middle::ty::{self, Ty, consts::ConstExt};

#[derive(Clone, Copy)]
struct Instance<'tcx> {
    definition: ty::AdtDef<'tcx>,
    arguments: ty::GenericArgsRef<'tcx>,
}

enum Failure {
    Unsupported(String),
    Budget(String),
}

fn unsupported(reason: &str) -> Failure {
    Failure::Unsupported(reason.into())
}

struct Capture<'a, 'tcx> {
    ctx: &'a Context<'tcx>,
    entries: usize,
    visiting: Vec<DefId>,
    origins: std::collections::BTreeMap<String, rstyle_pdg::declaration_origins::Origin>,
    generated_origins:
        std::collections::BTreeMap<String, rstyle_pdg::declaration_origins::GeneratedOrigin>,
    product_selections: Vec<rstyle_pdg::product_constant_values::SelectedUse>,
    reuse: reuse::Completed<'tcx>,
}

pub fn capture(ctx: &Context<'_>, facts: &mut HirFnFacts) -> Result<(), String> {
    if !has_data_interface(facts, ctx.profile) {
        return Ok(());
    }
    let mut capture = Capture {
        ctx,
        entries: 0,
        visiting: Vec::new(),
        origins: std::collections::BTreeMap::new(),
        generated_origins: std::collections::BTreeMap::new(),
        product_selections: Vec::new(),
        reuse: reuse::Completed::default(),
    };
    let parameters = facts
        .signature
        .params
        .iter()
        .map(|parameter| {
            let id = rustc_hir::HirId {
                owner: rustc_hir::OwnerId {
                    def_id: rustc_hir::def_id::LocalDefId {
                        local_def_index: rustc_hir::def_id::DefIndex::from_u32(
                            parameter.pat.hir.owner,
                        ),
                    },
                },
                local_id: rustc_hir::ItemLocalId::from_u32(parameter.pat.hir.local),
            };
            let ty = ctx
                .typeck
                .node_type_opt(id)
                .ok_or("Missing concrete parameter type")?;
            capture.observe(ty)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let observations = InterfaceObservations {
        identity: IDENTITY.into(),
        parameters,
        result: capture.observe(ctx.result)?,
    };
    observations.validate()?;
    capture.install_interface(facts, &observations)?;
    facts.build.frontend_options.insert_data(observations);

    Ok(())
}

fn has_data_interface(
    facts: &HirFnFacts,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    profile.supports("rust.concrete-read-only-data")
        || profile.supports("rust.verified-helper-calls")
            && (facts.signature.return_ty.normalized.is_none()
                || facts
                    .signature
                    .params
                    .iter()
                    .any(|parameter| parameter.ty.normalized.is_none()))
}

impl<'tcx> Capture<'_, 'tcx> {
    fn install_interface(
        mut self,
        facts: &mut HirFnFacts,
        interface: &InterfaceObservations,
    ) -> Result<(), String> {
        self.install_data(facts, interface)?;
        self.install_product_selections(facts)?;
        self.install_origins(interface, facts)
    }

    fn install_data(
        &mut self,
        facts: &mut HirFnFacts,
        interface: &InterfaceObservations,
    ) -> Result<(), String> {
        if self.ctx.profile.supports("rust.concrete-read-only-data") {
            self.install_tokens(facts, interface)?;
            self.install_receivers(facts)?;
            self.install_callback_environments(facts)?;
        }
        Ok(())
    }

    fn install_callback_environments(&mut self, facts: &mut HirFnFacts) -> Result<(), String> {
        if self.ctx.profile.supports("rust.immediate-option-callbacks") {
            self.callback_environments(&mut facts.body)?;
        }
        let ctx = self.ctx;
        ctx.capture_id_error_maps(&mut facts.body, &mut |ty| self.observe(ty))
    }

    fn install_tokens(
        &mut self,
        facts: &mut HirFnFacts,
        interface: &InterfaceObservations,
    ) -> Result<(), String> {
        interface_tokens(&mut facts.signature, interface)?;
        self.node_tokens(&mut facts.body, false)?;
        self.binding_tokens(&mut facts.bindings)
    }

    fn binding_tokens(&mut self, bindings: &mut [rstyle_pdg::BindingFact]) -> Result<(), String> {
        for binding in bindings {
            if binding.ty.normalized.is_none() {
                binding.ty.normalized = self.hir_token(&binding.hir, false)?;
            }
        }
        Ok(())
    }

    fn hir_token(
        &mut self,
        reference: &rstyle_pdg::HirRef,
        returning: bool,
    ) -> Result<Option<String>, String> {
        let id = rustc_hir::HirId {
            owner: rustc_hir::OwnerId {
                def_id: rustc_hir::def_id::LocalDefId {
                    local_def_index: rustc_hir::def_id::DefIndex::from_u32(reference.owner),
                },
            },
            local_id: rustc_hir::ItemLocalId::from_u32(reference.local),
        };
        let ty = if returning {
            Some(self.ctx.result)
        } else {
            self.ctx.typeck.node_type_opt(id)
        };
        let Some(ty) = ty else { return Ok(None) };
        self.type_token(ty)
    }

    fn node_tokens(
        &mut self,
        node: &mut rstyle_pdg::HirNode,
        index_bound: bool,
    ) -> Result<(), String> {
        self.value_token(node, index_bound)?;

        self.child_tokens(&mut node.children, node.kind == "expr_index")?;
        if matches!(node.kind.as_str(), "expr_method_call" | "expr_call")
            || node.kind == "expr_cast" && self.ctx.profile.supports("rust.fieldless-enum-values")
        {
            install_call_tokens(node);
        }
        Ok(())
    }

    fn value_token(
        &mut self,
        node: &mut rstyle_pdg::HirNode,
        index_bound: bool,
    ) -> Result<(), String> {
        self.product_constant(node)?;
        // Only index bounds retain the dedicated subslice proof grammar.
        // Standalone range receivers still need consistent concrete observations.
        let range_syntax = index_bound
            && node.kind == "expr_struct"
            && node
                .res
                .as_ref()
                .is_some_and(|resolution| resolution.kind == "resolved_range_type");
        if !range_syntax && node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none()) {
            let token = self.hir_token(&node.hir, node.kind == "expr_ret")?;
            node.ty.as_mut().expect("present type").normalized = token;
        }

        Ok(())
    }

    fn product_constant(&mut self, node: &mut rstyle_pdg::HirNode) -> Result<(), String> {
        if !self.ctx.profile.supports("rust.nominal-product-constants") {
            return Ok(());
        }
        let Some(definition) = super::nominal_constants::selected(self.ctx, node) else {
            return Ok(());
        };
        let ty = self
            .ctx
            .tcx
            .type_of(definition)
            .instantiate_identity()
            .skip_norm_wip();
        let observation = self.observe(ty)?;
        node.res = Some(match observation {
            Observation::Captured { ty } => {
                if let Some(observed) = node.ty.as_mut() {
                    observed.normalized = Some(rstyle_pdg::data_types::token(&ty)?);
                }
                let (resolution, selection) =
                    super::nominal_constants::resolution(self.ctx, definition, ty, node)?;
                if let Some(selection) = selection {
                    if self.product_selections.len()
                        >= rstyle_pdg::product_constant_values::MAX_USES
                    {
                        return Err("Product constant selections exceed use budget".into());
                    }
                    self.product_selections.push(selection);
                }
                resolution
            }
            Observation::Unsupported { reason } => {
                super::nominal_constants::unsupported_resolution(reason)
            }
        });

        Ok(())
    }

    fn install_product_selections(&mut self, facts: &mut HirFnFacts) -> Result<(), String> {
        if self.product_selections.is_empty() {
            return Ok(());
        }
        let observations = rstyle_pdg::product_constant_values::Selections {
            identity: rstyle_pdg::product_constant_values::IDENTITY.into(),
            uses: std::mem::take(&mut self.product_selections),
        };
        observations.validate()?;

        facts.build.frontend_options.insert(
            rstyle_pdg::product_constant_values::KEY.into(),
            serde_json::to_value(observations).map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    fn child_tokens(
        &mut self,
        nodes: &mut [rstyle_pdg::HirNode],
        indexing: bool,
    ) -> Result<(), String> {
        let bound = nodes
            .get(1)
            .filter(|_| indexing)
            .map(|node| node.hir.clone());
        for node in nodes {
            self.node_tokens(node, bound.as_ref() == Some(&node.hir))?;
        }
        Ok(())
    }

    fn install_origins(
        self,
        observations: &InterfaceObservations,
        facts: &mut HirFnFacts,
    ) -> Result<(), String> {
        let mut origins = rstyle_pdg::declaration_origins::Origins {
            identity: rstyle_pdg::declaration_origins::IDENTITY.into(),
            declarations: self.origins.into_values().collect(),
            generated: self.generated_origins.into_values().collect(),
        };
        let context = rstyle_pdg::declaration_origins::context(facts, observations, &origins)?;
        origins.retain_context(&context)?;
        rstyle_pdg::declaration_origins::verify(&context, &origins)?;
        facts.build.frontend_options.insert_data(origins);

        Ok(())
    }

    fn tree(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        self.entries += 1;
        if depth > MAX_DEPTH || self.entries > MAX_ENTRIES {
            return Err(Failure::Budget(format!(
                "Concrete type observation exceeds depth/entry budget (depth {depth}/{MAX_DEPTH}, entries {}/{MAX_ENTRIES})",
                self.entries
            )));
        }
        match ty.kind() {
            ty::Int(_) | ty::Uint(_) | ty::Bool => Ok(TypeTree::Scalar(ty.to_string())),
            ty::Char if depth == 0 && self.ctx.profile.supports("rust.character-scalars") => {
                Ok(TypeTree::Scalar("char".into()))
            }
            ty::Float(ty::FloatTy::F32 | ty::FloatTy::F64)
                if depth == 0 && self.ctx.profile.supports("rust.float-kernel-scalars") =>
            {
                Ok(TypeTree::Scalar(ty.to_string()))
            }
            ty::Tuple(elements) if elements.is_empty() => Ok(TypeTree::Scalar("unit".into())),
            ty::Str => Ok(TypeTree::Str),
            _ => self.composite(ty, depth),
        }
    }

    fn composite(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        match ty.kind() {
            ty::Ref(_, referent, Mutability::Mut)
                if depth == 0
                    && matches!(referent.kind(), ty::Uint(ty::UintTy::Usize))
                    && self.ctx.profile.supports("rust.mutable-scalar-formals") =>
            {
                Ok(TypeTree::MutableScalar("usize".into()))
            }
            ty::Tuple(elements) => self.tuple(elements, depth),
            ty::Ref(_, referent, Mutability::Not) | ty::Slice(referent) => {
                self.referent(*referent, depth, matches!(ty.kind(), ty::Slice(_)))
            }
            _ => self.storage(ty, depth),
        }
    }

    fn tuple(&mut self, elements: &ty::List<Ty<'tcx>>, depth: usize) -> Result<TypeTree, Failure> {
        elements
            .iter()
            .map(|element| self.tree(element, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(TypeTree::Tuple)
    }

    fn referent(&mut self, ty: Ty<'tcx>, depth: usize, slice: bool) -> Result<TypeTree, Failure> {
        let referent = Box::new(self.tree(ty, depth + 1)?);

        Ok(if slice {
            TypeTree::Slice(referent)
        } else {
            TypeTree::Shared(referent)
        })
    }

    fn storage(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        if let Some(nonzero) = self.nonzero_value(ty)? {
            return Ok(nonzero);
        }
        if let Some(range) = self.range_value(ty, depth)? {
            return Ok(range);
        }

        if self.ctx.profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
            && let Some(payload) = super::options::payload(self.ctx.tcx, ty)
                .or_else(|| self.nonnested_option_payload(ty))
        {
            return self.option_type(payload, depth);
        }
        if let Some(result) = self.int_conversion_result(ty) {
            return Ok(result);
        }
        if let Some(result) = self.slice_array_result(ty) {
            return Ok(result);
        }
        if self.ctx.profile.supports("rust.fresh-concrete-results")
            && self.visiting.is_empty()
            && let Some((ok, err)) = super::results::payloads(self.ctx.tcx, ty)
        {
            return self.result_type(ok, err, depth);
        }
        self.storage_container(ty, depth)
    }

    fn nonnested_option_payload(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        self.visiting.is_empty().then(|| {
            super::options::product_payload(self.ctx, ty)
                .or_else(|| super::options::borrowed_slice_payload(self.ctx, ty))
        })?
    }

    fn option_type(&mut self, payload: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        let option = TypeTree::Option(Box::new(self.tree(payload, depth + 1)?));
        rstyle_pdg::concrete_types::validate(&option).map_err(Failure::Unsupported)?;
        Ok(option)
    }

    fn result_type(
        &mut self,
        ok: Ty<'tcx>,
        err: Ty<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let ok = self.tree(ok, depth + 1)?;
        // Resolve only the associated error of this observational product shape.
        // Source/member authority and guarded value admission remain separate.
        let err = if matches!(err.kind(), ty::Alias(..))
            && observed_static_string_result_success(&ok)
        {
            let environment =
                ty::TypingEnv::post_analysis(self.ctx.tcx, self.ctx.typeck.hir_owner.def_id);
            self.ctx
                .tcx
                .try_normalize_erasing_regions(environment, ty::Unnormalized::new_wip(err))
                .map_err(|_| unsupported("Static product associated error cannot be resolved"))?
        } else {
            err
        };
        let result = TypeTree::Result {
            ok: Box::new(ok),
            err: Box::new(self.tree(err, depth + 1)?),
        };
        rstyle_pdg::concrete_types::validate(&result).map_err(Failure::Unsupported)?;

        Ok(result)
    }

    fn storage_container(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        match ty.kind() {
            ty::Array(element, length) => self.array(*element, *length, depth),
            ty::Adt(definition, arguments) => self.nominal_type(*definition, arguments, depth),
            _ => Err(unsupported(
                "Outside the concrete drop-free/shared type observation grammar",
            )),
        }
    }

    fn nominal_type(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        if definition.is_enum() && self.ctx.profile.supports("rust.scalar-enum-values") {
            self.enumeration(definition, arguments, depth)
        } else {
            self.structure(definition, arguments, depth)
        }
    }

    fn enumeration(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        if self.ctx.profile.supports("rust.fieldless-enum-values")
            && definition
                .variants()
                .iter()
                .all(|variant| variant.fields.is_empty())
        {
            self.fieldless_enum(definition, arguments)
        } else if self.ctx.profile.supports("rust.borrowed-mixed-owners")
            && definition
                .variants()
                .iter()
                .any(|variant| variant.fields.is_empty() || variant.ctor_kind().is_none())
        {
            self.mixed_enum(definition, arguments, depth)
        } else {
            self.scalar_enum(definition, arguments, depth)
        }
    }

    fn int_conversion_result(&self, value: Ty<'tcx>) -> Option<TypeTree> {
        if !self.ctx.profile.supports("rust.bitset-cardinality") {
            return None;
        }
        let ty::Adt(definition, arguments) = value.kind() else {
            return None;
        };
        let tcx = self.ctx.tcx;

        if !tcx.is_diagnostic_item(rustc_span::sym::Result, definition.did())
            || !(arguments.type_at(0) == tcx.types.usize
                || self.ctx.profile.supports("rust.end-of-input")
                    && arguments.type_at(0) == tcx.types.u16
                || self.ctx.profile.supports("rust.lossless-usize-u64-calls")
                    && arguments.type_at(0) == tcx.types.u64)
        {
            return None;
        }
        let ty::Adt(error, _) = arguments.type_at(1).kind() else {
            return None;
        };
        let path = tcx.def_path_str(error.did());
        (tcx.crate_name(error.did().krate).as_str() == "core"
            && matches!(
                path.as_str(),
                "core::num::error::TryFromIntError"
                    | "core::num::TryFromIntError"
                    | "std::num::TryFromIntError"
            ))
        .then(|| TypeTree::IntConversionResult(arguments.type_at(0).to_string()))
    }

    fn slice_array_result(&self, value: Ty<'tcx>) -> Option<TypeTree> {
        if !self.ctx.profile.supports("rust.slice-array-byte-calls") {
            return None;
        }
        let ty::Adt(definition, arguments) = value.kind() else {
            return None;
        };
        let tcx = self.ctx.tcx;

        if !tcx.is_diagnostic_item(rustc_span::sym::Result, definition.did()) {
            return None;
        }
        let ty::Adt(error, _) = arguments.type_at(1).kind() else {
            return None;
        };
        if tcx.crate_name(error.did().krate).as_str() != "core"
            || !tcx
                .def_path_str(error.did())
                .ends_with("::array::TryFromSliceError")
        {
            return None;
        }
        let (array, borrowed) = match arguments.type_at(0).kind() {
            ty::Ref(_, array, Mutability::Not) => (*array, true),
            _ => (arguments.type_at(0), false),
        };
        let ty::Array(element, length) = array.kind() else {
            return None;
        };
        if !matches!(element.kind(), ty::Uint(ty::UintTy::U8)) {
            return None;
        }
        let length = length.try_to_target_usize(tcx)?;

        (length <= MAX_FIELDS as u64).then_some(TypeTree::SliceArrayResult { length, borrowed })
    }

    fn array(
        &mut self,
        element: Ty<'tcx>,
        length: ty::Const<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let length = length
            .try_to_target_usize(self.ctx.tcx)
            .ok_or_else(|| unsupported("Array length is not a resolved concrete value"))?;

        Ok(TypeTree::Array {
            element: Box::new(self.tree(element, depth + 1)?),
            length,
        })
    }

    fn structure(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let instance = Instance {
            definition,
            arguments,
        };
        if let Some(captured) = self.scalar_product_instance(instance, depth)? {
            return Ok(captured);
        }
        self.check_structure(instance)?;
        if let Some(generated) = generated_declarations::capture(self, instance, depth)? {
            return Ok(generated);
        }

        self.ordinary_structure(instance, depth)
    }

    fn ordinary_structure(
        &mut self,
        instance: Instance<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let definition = instance.definition;
        let source = self.concrete_declaration(instance)?;
        let fields = &definition.non_enum_variant().fields;
        let declarations = field_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != fields.len() {
            return Err(unsupported("Concrete field/source inventory disagrees"));
        }
        let captured = self.struct_fields(instance, declarations, depth);

        let nominal = nominal(
            self.ctx,
            self.ctx.tcx.def_path_str(definition.did()),
            &source,
        )?;
        self.origin(definition.did(), &nominal)?;

        Ok(TypeTree::Struct {
            nominal,
            source,
            fields: captured?,
        })
    }

    fn check_structure(&self, instance: Instance<'tcx>) -> Result<(), Failure> {
        let Instance {
            definition,
            arguments,
        } = instance;
        let tcx = self.ctx.tcx;
        if self.ctx.profile.is_data()
            && arguments.len() > MAX_FIELDS
            && arguments
                .iter()
                .all(|argument| matches!(argument.kind(), ty::GenericArgKind::Lifetime(_)))
        {
            return Err(Failure::Budget(
                "Nominal lifetime arguments exceed count budget".into(),
            ));
        }

        if !definition.is_struct()
            || !definition.did().is_local()
            || !self.supported_arguments(arguments)
            || definition.has_dtor(tcx)
            || tcx.lang_items().unsafe_cell_type() == Some(definition.did())
        {
            return Err(unsupported(
                "Struct requires local concrete source, no generics/destructor/interior mutability",
            ));
        }
        if self.visiting.contains(&definition.did()) {
            return Err(unsupported(
                "Recursive concrete type expansion is unavailable",
            ));
        }
        if definition.non_enum_variant().fields.len() > MAX_FIELDS {
            return Err(Failure::Budget(
                "Concrete struct exceeds field budget".into(),
            ));
        }
        Ok(())
    }

    fn concrete_declaration(
        &self,
        instance: Instance<'tcx>,
    ) -> Result<rstyle_pdg::SourceText, Failure> {
        let Instance {
            definition,
            arguments,
        } = instance;
        let source = self.declaration(definition.did())?;
        if self.ctx.profile.is_data() {
            let count = lifetimes::declaration_count(&source).map_err(Failure::Unsupported)?;
            if count != arguments.len() {
                return Err(unsupported(
                    "Nominal declaration/resolved lifetime arity disagrees",
                ));
            }
        }
        Ok(source)
    }

    fn struct_fields(
        &mut self,
        instance: Instance<'tcx>,
        declarations: Vec<(String, String)>,
        depth: usize,
    ) -> Result<Vec<Field>, Failure> {
        self.visiting.push(instance.definition.did());
        let captured = self.fields(instance, declarations, depth);
        self.visiting.pop();

        captured
    }

    fn supported_arguments(&self, arguments: ty::GenericArgsRef<'tcx>) -> bool {
        arguments.is_empty()
            || self.ctx.profile.is_data()
                && arguments
                    .iter()
                    .all(|argument| matches!(argument.kind(), ty::GenericArgKind::Lifetime(_)))
    }

    fn origin(&mut self, id: DefId, nominal: &Nominal) -> Result<(), Failure> {
        let Node::Item(node) = self.ctx.tcx.hir_node_by_def_id(id.expect_local()) else {
            return Err(unsupported("Missing source struct item"));
        };
        let item = node.span;
        let file = self.ctx.tcx.sess.source_map().lookup_source_file(item.lo());
        let rustc_span::FileName::Real(real) = &file.name else {
            return Err(unsupported("Concrete declaration lacks real source file"));
        };
        let path = real
            .local_path()
            .ok_or_else(|| unsupported("Missing local declaration file"))?;
        let snapshot = self
            .ctx
            .sources
            .file(&file)
            .ok_or_else(|| unsupported("Missing original declaration file buffer"))?;
        let (_, declaration_span) = self
            .ctx
            .sources
            .range(self.ctx.tcx, item)
            .ok_or_else(|| unsupported("Missing original declaration range"))?;
        self.origins.insert(
            nominal.definition_path.clone(),
            rstyle_pdg::declaration_origins::Origin {
                nominal: nominal.clone(),
                source_file: if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    std::env::current_dir()
                        .map_err(|_| unsupported("Missing compiler working directory"))?
                        .join(path)
                }
                .to_string_lossy()
                .into_owned(),
                file_digest: snapshot.checksum().to_owned(),
                file_bytes: snapshot.bytes.clone(),
                declaration_span,
                builtin_copy: copy::observe(self.ctx, id),
            },
        );

        Ok(())
    }

    fn declaration(&self, id: DefId) -> Result<rstyle_pdg::SourceText, Failure> {
        let Node::Item(item) = self.ctx.tcx.hir_node_by_def_id(id.expect_local()) else {
            return Err(unsupported("Missing original struct declaration"));
        };
        if item.span.from_expansion() {
            return Err(unsupported(
                "Macro-generated struct declaration is unavailable",
            ));
        }
        let (source, _) = self
            .ctx
            .sources
            .range(self.ctx.tcx, item.span)
            .ok_or_else(|| unsupported("Missing original concrete declaration bytes"))?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(Failure::Budget(
                "Concrete declaration exceeds source budget".into(),
            ));
        }

        Ok(source)
    }

    fn fields(
        &mut self,
        instance: Instance<'tcx>,
        declarations: Vec<(String, String)>,
        depth: usize,
    ) -> Result<Vec<Field>, Failure> {
        instance
            .definition
            .non_enum_variant()
            .fields
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (field, (name, declaration)))| {
                if field.name.to_string() != name {
                    return Err(unsupported(
                        "Resolved field identity disagrees with original declaration",
                    ));
                }
                let tcx = self.ctx.tcx;
                let environment =
                    ty::TypingEnv::post_analysis(tcx, self.ctx.typeck.hir_owner.def_id);
                let resolved = tcx
                    .try_normalize_erasing_regions(
                        environment,
                        tcx.type_of(field.did).instantiate(tcx, instance.arguments),
                    )
                    .map_err(|_| unsupported("Concrete field type cannot be resolved"))?;
                let ty = self.tree(resolved, depth + 1)?;
                Ok(Field {
                    position,
                    name,
                    declaration: declaration.into(),
                    ty,
                })
            })
            .collect()
    }
}

fn nominal(
    ctx: &Context<'_>,
    definition_path: String,
    source: &rstyle_pdg::SourceText,
) -> Result<Nominal, Failure> {
    if ctx.profile.is_data() {
        let context = ctx
            .unit_context
            .as_ref()
            .ok_or_else(|| unsupported("Missing explicit selected-unit context"))?;
        return Ok(Nominal {
            package_name: context.package.name.clone(),
            package_version: context.package.version.clone(),
            manifest_digest: context.package.manifest_digest.clone(),
            source_identity: Some(context.package.package_id.clone()),
            definition_path,
            declaration_digest: source.checksum().to_owned(),
        });
    }
    let package_name = std::env::var("CARGO_PKG_NAME")
        .map_err(|_| unsupported("Missing Cargo package identity"))?;
    let package_version = std::env::var("CARGO_PKG_VERSION")
        .map_err(|_| unsupported("Missing Cargo package version"))?;
    let directory = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| unsupported("Missing Cargo manifest context"))?;
    let manifest = ctx
        .sources
        .manifest(&std::path::Path::new(&directory).join("Cargo.toml"))
        .map_err(|_| unsupported("Missing exact Cargo manifest bytes"))?;

    Ok(Nominal {
        package_name,
        source_identity: None,
        package_version,
        manifest_digest: manifest.checksum().to_owned(),
        definition_path,
        declaration_digest: source.checksum().to_owned(),
    })
}

pub(super) fn static_owner(
    ctx: &Context<'_>,
    implementation: DefId,
) -> Result<Option<Nominal>, String> {
    let ty = ctx
        .tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_norm_wip();

    let ty::Adt(definition, _) = ty.kind() else {
        return Ok(None);
    };
    if !definition.is_struct() || !definition.did().is_local() {
        return Ok(None);
    }
    let Node::Item(item) = ctx.tcx.hir_node_by_def_id(definition.did().expect_local()) else {
        return Ok(None);
    };
    if item.span.from_expansion() {
        return Ok(None);
    }

    let Some((source, _)) = ctx.sources.range(ctx.tcx, item.span) else {
        return Ok(None);
    };
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Static owner declaration exceeds source budget".into());
    }
    match nominal(ctx, ctx.tcx.def_path_str(definition.did()), &source) {
        Ok(owner) => Ok(Some(owner)),
        Err(Failure::Unsupported(_)) => Ok(None),
        Err(Failure::Budget(reason)) => Err(reason),
    }
}

fn interface_tokens(
    signature: &mut rstyle_pdg::SignatureFacts,
    interface: &InterfaceObservations,
) -> Result<(), String> {
    for (parameter, observation) in signature.params.iter_mut().zip(&interface.parameters) {
        if let Observation::Captured { ty } = observation {
            parameter.ty.normalized = Some(rstyle_pdg::data_types::token(ty)?);
            parameter.pat.ty = Some(parameter.ty.clone());
        }
    }

    if let Observation::Captured { ty } = &interface.result {
        signature.return_ty.normalized = Some(rstyle_pdg::data_types::token(ty)?);
    }
    Ok(())
}

fn install_call_tokens(node: &mut rstyle_pdg::HirNode) {
    let arguments: Option<Vec<_>> = node
        .children
        .iter()
        .map(|child| child.ty.as_ref()?.normalized.clone())
        .collect();
    let result = node.ty.as_ref().and_then(|ty| ty.normalized.clone());
    if let (Some(dispatch), Some(arguments), Some(result)) = (&mut node.dispatch, arguments, result)
    {
        dispatch.operand_tys = arguments;
        dispatch.result_ty = result;
    }
}

pub(super) use scalar_product_instances::scalar_argument;
