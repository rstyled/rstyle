//! Concrete same-unit inherent ownership, captured before helper linking.

mod source_generics;

use rstyle_pdg::inherent_calls::CallOwner;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, TyCtxt};

pub fn implementation(
    tcx: TyCtxt<'_>,
    id: DefId,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Option<DefId> {
    if !profile.supports("rust.verified-inherent-helper-calls")
        || !id.is_local()
        || tcx.def_kind(id) != rustc_hir::def::DefKind::AssocFn
    {
        return None;
    }
    let owner = tcx.inherent_impl_of_assoc(id)?;
    let nongeneric = tcx.generics_of(id).count() == 0
        && tcx.generics_of(owner).count() == 0
        && !source_generics::has_lifetime_arguments(tcx, owner);
    (nongeneric
        || profile.supports("rust.borrowed-mixed-owners")
            && lifetime_owner(
                tcx,
                id,
                owner,
                profile.supports("rust.linear-product-result-flow"),
            ))
    .then_some(owner)
}

fn lifetime_owner(tcx: TyCtxt<'_>, id: DefId, owner: DefId, static_product: bool) -> bool {
    let method = tcx.generics_of(id);
    let implementation = tcx.generics_of(owner);
    method.own_params.is_empty()
        && method.parent == Some(owner)
        && implementation.parent.is_none()
        && !implementation.own_params.is_empty()
        && implementation.own_params.len() <= rstyle_pdg::concrete_types::MAX_FIELDS
        && implementation
            .own_params
            .iter()
            .all(|parameter| matches!(parameter.kind, ty::GenericParamDefKind::Lifetime))
        && source_generics::implementation(tcx, owner)
        && if tcx.associated_item(id).is_method() {
            matches!(tcx.fn_sig(id).instantiate_identity().skip_binder().inputs().first(),
                Some(receiver) if matches!(receiver.kind(), ty::Ref(_, _, rustc_hir::Mutability::Not)))
        } else {
            static_product
        }
}

pub fn call_owner(tcx: TyCtxt<'_>, owner: DefId) -> CallOwner {
    CallOwner {
        identity: rstyle_pdg::inherent_calls::IDENTITY.into(),
        implementation_identity: format!("{:?}", tcx.def_path_hash(owner)),
    }
}
