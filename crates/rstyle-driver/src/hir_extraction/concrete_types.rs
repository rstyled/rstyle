//! Observational type capture for the unavailable data tier. No eligibility flag.

mod receivers;

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
    capture.install_data(facts, &observations)?;
    capture.install_origins(&observations, facts)?;
    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(observations).map_err(|error| error.to_string())?,
    );

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
    fn install_data(
        &mut self,
        facts: &mut HirFnFacts,
        interface: &InterfaceObservations,
    ) -> Result<(), String> {
        if self.ctx.profile.supports("rust.concrete-read-only-data") {
            self.install_tokens(facts, interface)?;
            self.install_receivers(facts)?;
        }
        Ok(())
    }

    fn install_tokens(
        &mut self,
        facts: &mut HirFnFacts,
        interface: &InterfaceObservations,
    ) -> Result<(), String> {
        interface_tokens(&mut facts.signature, interface)?;
        self.node_tokens(&mut facts.body)?;
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
        match self.observe(ty)? {
            Observation::Captured { ty } => rstyle_pdg::data_types::token(&ty).map(Some),
            Observation::Unsupported { .. } => Ok(None),
        }
    }

    fn node_tokens(&mut self, node: &mut rstyle_pdg::HirNode) -> Result<(), String> {
        if node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none()) {
            let token = self.hir_token(&node.hir, node.kind == "expr_ret")?;
            node.ty.as_mut().expect("present type").normalized = token;
        }
        self.child_tokens(&mut node.children)?;
        if matches!(node.kind.as_str(), "expr_method_call" | "expr_call") {
            install_call_tokens(node);
        }
        Ok(())
    }

    fn child_tokens(&mut self, nodes: &mut [rstyle_pdg::HirNode]) -> Result<(), String> {
        for node in nodes {
            self.node_tokens(node)?;
        }
        Ok(())
    }

    fn install_origins(
        mut self,
        observations: &InterfaceObservations,
        facts: &mut HirFnFacts,
    ) -> Result<(), String> {
        let expected = rstyle_pdg::declaration_origins::declarations(observations)?;
        self.origins.retain(|key, _| expected.contains_key(key));
        let origins = rstyle_pdg::declaration_origins::Origins {
            identity: rstyle_pdg::declaration_origins::IDENTITY.into(),
            declarations: self.origins.into_values().collect(),
        };
        rstyle_pdg::declaration_origins::verify(observations, &origins)?;
        facts.build.frontend_options.insert(
            rstyle_pdg::declaration_origins::KEY.into(),
            serde_json::to_value(origins).map_err(|error| error.to_string())?,
        );

        Ok(())
    }

    fn observe(&mut self, ty: Ty<'tcx>) -> Result<Observation, String> {
        match self.tree(ty, 0) {
            Ok(ty) => Ok(Observation::Captured { ty }),
            Err(Failure::Unsupported(reason)) => Ok(Observation::Unsupported { reason }),
            Err(Failure::Budget(reason)) => Err(reason),
        }
    }

    fn tree(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        self.entries += 1;
        if depth > MAX_DEPTH || self.entries > MAX_ENTRIES {
            return Err(Failure::Budget(
                "Concrete type observation exceeds depth/entry budget".into(),
            ));
        }
        match ty.kind() {
            ty::Int(_) | ty::Uint(_) | ty::Bool => Ok(TypeTree::Scalar(ty.to_string())),
            ty::Tuple(elements) if elements.is_empty() => Ok(TypeTree::Scalar("unit".into())),
            ty::Str => Ok(TypeTree::Str),
            _ => self.composite(ty, depth),
        }
    }

    fn composite(&mut self, ty: Ty<'tcx>, depth: usize) -> Result<TypeTree, Failure> {
        match ty.kind() {
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
        match ty.kind() {
            ty::Array(element, length) => self.array(*element, *length, depth),
            ty::Adt(definition, arguments) => self.structure(*definition, arguments, depth),
            _ => Err(unsupported(
                "Outside the concrete drop-free/shared type observation grammar",
            )),
        }
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
        if length > MAX_ARRAY {
            return Err(Failure::Budget(
                "Concrete array exceeds length budget".into(),
            ));
        }

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
        self.check_structure(instance)?;
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
        if self.ctx.profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV1
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

    fn concrete_declaration(&self, instance: Instance<'tcx>) -> Result<String, Failure> {
        let Instance {
            definition,
            arguments,
        } = instance;
        let source = self.declaration(definition.did())?;
        if self.ctx.profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV1 {
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
            || self.ctx.profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV1
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
        let full = rustc_span::Span::with_root_ctxt(file.start_pos, file.end_position());
        let (mut file_bytes, file_span) = super::original_source(self.ctx.tcx, full)
            .ok_or_else(|| unsupported("Missing original declaration file buffer"))?;
        // The compiler's full normalized span starts after a removed BOM;
        // restore that prefix so original file-relative offsets stay exact.
        if file_span.start == 3 {
            file_bytes.insert(0, '\u{feff}');
        } else if file_span.start != 0 {
            return Err(unsupported("Unexpected original file prefix offset"));
        }
        let (_, declaration_span) = super::original_source(self.ctx.tcx, item)
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
                file_digest: digest(file_bytes.as_bytes()),
                file_bytes,
                declaration_span,
            },
        );

        Ok(())
    }

    fn declaration(&self, id: DefId) -> Result<String, Failure> {
        let Node::Item(item) = self.ctx.tcx.hir_node_by_def_id(id.expect_local()) else {
            return Err(unsupported("Missing original struct declaration"));
        };
        if item.span.from_expansion() {
            return Err(unsupported(
                "Macro-generated struct declaration is unavailable",
            ));
        }
        let (source, _) = super::original_source(self.ctx.tcx, item.span)
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
                    declaration,
                    ty,
                })
            })
            .collect()
    }
}

fn nominal(ctx: &Context<'_>, definition_path: String, source: &str) -> Result<Nominal, Failure> {
    if ctx.profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV1 {
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
            declaration_digest: digest(source.as_bytes()),
        });
    }
    let package_name = std::env::var("CARGO_PKG_NAME")
        .map_err(|_| unsupported("Missing Cargo package identity"))?;
    let package_version = std::env::var("CARGO_PKG_VERSION")
        .map_err(|_| unsupported("Missing Cargo package version"))?;
    let directory = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| unsupported("Missing Cargo manifest context"))?;
    let manifest = std::fs::read(std::path::Path::new(&directory).join("Cargo.toml"))
        .map_err(|_| unsupported("Missing exact Cargo manifest bytes"))?;

    Ok(Nominal {
        package_name,
        source_identity: None,
        package_version,
        manifest_digest: digest(&manifest),
        definition_path,
        declaration_digest: digest(source.as_bytes()),
    })
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
