//! Observational type capture for the unavailable data tier. No eligibility flag.

use super::hir::Context;
use rstyle_pdg::{HirFnFacts, concrete_types::*};
use rustc_hir::{Mutability, Node, def_id::DefId};
use rustc_middle::ty::{self, Ty, consts::ConstExt};

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
}

pub fn capture(ctx: &Context<'_>, facts: &mut HirFnFacts) -> Result<(), String> {
    if !has_data_interface(facts, ctx.profile) {
        return Ok(());
    }
    let mut capture = Capture {
        ctx,
        entries: 0,
        visiting: Vec::new(),
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
    profile.supports("rust.verified-helper-calls")
        && (facts.signature.return_ty.normalized.is_none()
            || facts
                .signature
                .params
                .iter()
                .any(|parameter| parameter.ty.normalized.is_none()))
}

impl<'tcx> Capture<'_, 'tcx> {
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
        let tcx = self.ctx.tcx;
        if !definition.is_struct()
            || !definition.did().is_local()
            || !arguments.is_empty()
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
        let fields = &definition.non_enum_variant().fields;
        if fields.len() > MAX_FIELDS {
            return Err(Failure::Budget(
                "Concrete struct exceeds field budget".into(),
            ));
        }
        let source = self.declaration(definition.did())?;
        let declarations = field_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != fields.len() {
            return Err(unsupported("Concrete field/source inventory disagrees"));
        }
        self.visiting.push(definition.did());
        let captured = self.fields(fields.iter(), declarations, depth);
        self.visiting.pop();

        Ok(TypeTree::Struct {
            nominal: nominal(tcx.def_path_str(definition.did()), &source)?,
            source,
            fields: captured?,
        })
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

    fn fields<'a>(
        &mut self,
        fields: impl Iterator<Item = &'a ty::FieldDef>,
        declarations: Vec<(String, String)>,
        depth: usize,
    ) -> Result<Vec<Field>, Failure> {
        fields
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
                        tcx.type_of(field.did).instantiate_identity(),
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

fn nominal(definition_path: String, source: &str) -> Result<Nominal, Failure> {
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
        package_version,
        manifest_digest: digest(&manifest),
        definition_path,
        declaration_digest: digest(source.as_bytes()),
    })
}
