//! Standard-library identity by crate number, never by crate or path spelling.
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{CrateNum, DefId, LocalDefId};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;
use rustc_type_ir::elaborate::supertrait_def_ids;
use std::collections::HashSet;

/// `core`, `alloc` or `std`, identified by the crates defining their lang and diagnostic items.
pub(crate) fn sysroot(tcx: ty::TyCtxt<'_>, krate: CrateNum) -> bool {
    let anchors = [
        tcx.lang_items().sized_trait(),
        tcx.lang_items().owned_box(),
        tcx.get_diagnostic_item(sym::HashMap),
    ];
    anchors
        .into_iter()
        .flatten()
        .any(|item| item.krate == krate)
}

/// The types `ty` is built from, leaving out a closure written in `owner`, when there is one, and
/// what it's built from: its signature and captures are the owner's own, and so is what it does.
fn parts<'tcx>(tcx: ty::TyCtxt<'tcx>, owner: Option<DefId>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    let mut walker = ty.walk();
    let mut found = Vec::new();
    while let Some(part) = walker.next() {
        let Some(ty) = part.as_type() else {
            continue;
        };
        if let ty::Closure(closure, _) = ty.kind()
            && Some(tcx.typeck_root_def_id(*closure)) == owner
        {
            walker.skip_current_subtree();
            continue;
        }
        found.push(ty);
    }
    found
}

fn standard_kind<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(definition, _) => sysroot(tcx, definition.did().krate),
        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_)
        | ty::Str
        | ty::Array(..)
        | ty::Slice(_)
        | ty::RawPtr(..)
        | ty::Ref(..)
        | ty::Never
        | ty::Tuple(_) => true,
        _ => false,
    }
}

/// A standard callee may run workspace code through callable arguments, or through an
/// implementation one of its bounds selects, such as `PartialEq::ne`, `Into::into` or `sort` on
/// workspace items. With `own_closures` left out, a closure written in the `owner` isn't counted:
/// its body is the owner's own.
pub(crate) fn calls_back<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (owner, own_closures): (LocalDefId, bool),
    callee: DefId,
    arguments: ty::GenericArgsRef<'tcx>,
) -> bool {
    let own = (!own_closures).then(|| owner.to_def_id());
    let callable = arguments.types().any(|ty| {
        parts(tcx, own, ty).into_iter().any(|ty| {
            matches!(
                ty.kind(),
                ty::Closure(..)
                    | ty::FnDef(..)
                    | ty::FnPtr(..)
                    | ty::Dynamic(..)
                    | ty::Param(_)
                    | ty::Alias(..)
                    | ty::Coroutine(..)
            )
        })
    });
    let mut bounds = Bounds {
        tcx,
        environment: ty::TypingEnv::post_analysis(tcx, owner),
        own,
        seen: HashSet::new(),
    };
    callable || !bounds.standard(callee, arguments)
}

/// The implementations a call's bounds select, each read once.
struct Bounds<'tcx> {
    tcx: ty::TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    /// The function whose closures are its own code, when they aren't counted.
    own: Option<DefId>,
    seen: HashSet<ty::TraitRef<'tcx>>,
}

impl<'tcx> Bounds<'tcx> {
    /// Whether every bound of `item` selects only the standard library's code.
    fn standard(&mut self, item: DefId, arguments: ty::GenericArgsRef<'tcx>) -> bool {
        let clauses = self.tcx.clauses_of(item).instantiate(self.tcx, arguments);
        clauses
            .clauses
            .into_iter()
            .filter_map(|clause| clause.skip_norm_wip().as_trait_clause())
            .all(|bound| self.selects_standard(bound.map_bound(|bound| bound.trait_ref)))
    }

    /// A bound on an associated type is on the type it names, as `Self::Item: Ord` is on the
    /// item's. One that names none here could select anything.
    fn selects_standard(&mut self, bound: ty::PolyTraitRef<'tcx>) -> bool {
        let bound = self.tcx.instantiate_bound_regions_with_erased(bound);
        let bound = ty::Unnormalized::new_wip(bound);
        let named = self
            .tcx
            .try_normalize_erasing_regions(self.environment, bound);
        named.is_ok_and(|bound| self.implemented_by_standard(bound))
    }

    /// A bound is read by the implementation it selects, as a call is. The standard library's
    /// runs what its own bounds select, so `Iter<Item>: Iterator` runs nothing of `Item`'s, and
    /// `Vec<Item>: Clone` runs `Item`'s `clone`. A trait with no function to call selects no
    /// code. One the compiler implements runs what the type is built from, and calling a closure
    /// runs the closure.
    fn implemented_by_standard(&mut self, bound: ty::TraitRef<'tcx>) -> bool {
        let tcx = self.tcx;
        if !declares_functions(tcx, bound.def_id) || !self.seen.insert(bound) {
            return true;
        }
        match tcx.codegen_select_candidate(self.environment.as_query_input(bound)) {
            Ok(ImplSource::UserDefined(found)) if sysroot(tcx, found.impl_def_id.krate) => {
                self.standard(found.impl_def_id, found.args)
            }
            Ok(ImplSource::Builtin(..)) => {
                let own = self.own.filter(|_| tcx.is_fn_trait(bound.def_id));
                let parts = parts(tcx, own, bound.self_ty());
                parts.into_iter().all(|ty| standard_kind(tcx, ty))
            }
            _ => false,
        }
    }
}

/// Whether the trait, or one it requires, declares a function an implementation defines.
fn declares_functions(tcx: ty::TyCtxt<'_>, declared: DefId) -> bool {
    supertrait_def_ids(tcx, declared).any(|required| {
        let mut items = tcx.associated_items(required).in_definition_order();
        items.any(|item| item.is_fn())
    })
}

/// Contract namespace of this frontend's operation keys.
const NAMESPACE: &str = "rust";

/// Language-neutral key of a standard operation, matched against library contracts:
/// the defining impl's self type and the member name, as in `rust:BTreeMap.get`.
/// Never derived from module path spelling, except for a trait implementation's member
/// whose self type has no compiler name, which `implementation_operation` keys.
pub(crate) fn operation(tcx: ty::TyCtxt<'_>, callee: DefId) -> Option<String> {
    let implementation = tcx.impl_of_assoc(callee)?;
    let self_ty = tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    match type_key(tcx, self_ty) {
        Some(owner) => Some(key(&owner, tcx, callee)),
        None => implementation_operation(tcx, callee),
    }
}

/// Key of a standard free function by the path it's defined at, as its stable identity names
/// it: `rust:core::mem::take`, though a caller writes `std::mem::take`. It has no type to be
/// keyed by, and few have a diagnostic item. One declared outside a module, as in a function
/// body or a foreign block, has no key.
pub(crate) fn function_operation(tcx: ty::TyCtxt<'_>, function: DefId) -> Option<String> {
    let parent = tcx.opt_parent(function)?;
    (tcx.def_kind(parent) == DefKind::Mod).then(|| path_key(tcx, function))
}

/// Key of a trait implementation's member whose self type has no compiler name, such as the
/// `T` of a blanket implementation, `&Vec<T>`, `[T; N]` or `vec::IntoIter<T>`, by the path it's
/// defined at, as a free function is: `rust:alloc::string::{impl#39}::to_string` is the
/// `ToString` every `to_string` runs. A call that selects it is classified by it. An inherent
/// member of such a type still has no key.
fn implementation_operation(tcx: ty::TyCtxt<'_>, member: DefId) -> Option<String> {
    let item = tcx.opt_associated_item(member)?;
    item.trait_item_def_id().map(|_| path_key(tcx, member))
}

fn path_key(tcx: ty::TyCtxt<'_>, definition: DefId) -> String {
    format!("{NAMESPACE}:{}", crate::ids::def_id_str(tcx, definition))
}

/// Key of a standard trait's own member by the trait's diagnostic item name, as in
/// `rust:Iterator.map`; the selected impl does not change the member's role.
pub(crate) fn trait_operation(tcx: ty::TyCtxt<'_>, owner: DefId, member: DefId) -> Option<String> {
    let name = tcx.get_diagnostic_name(owner)?;
    Some(key(name.as_str(), tcx, member))
}

fn key(owner: &str, tcx: ty::TyCtxt<'_>, member: DefId) -> String {
    format!("{NAMESPACE}:{owner}.{}", tcx.item_name(member))
}

/// A standard type by its compiler-registered name: diagnostic item, then lang item, or
/// the primitive kind. Unnamed and generic self types have no key.
pub(crate) fn type_key(tcx: ty::TyCtxt<'_>, ty: Ty<'_>) -> Option<String> {
    match ty.kind() {
        ty::Adt(definition, _) => {
            let id = definition.did();
            tcx.get_diagnostic_name(id)
                .or_else(|| tcx.as_lang_item(id).map(|item| item.name()))
                .map(|name| name.to_string())
        }
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => Some(ty.to_string()),
        ty::Slice(_) => Some("slice".into()),
        ty::Str => Some("str".into()),
        _ => None,
    }
}
