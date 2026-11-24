//! A macro expansion's identity, without the machine's paths.
//!
//! rustc names an expansion by its `ExpnHash`, which hashes the expansion's spans together with
//! their source files' names, and a file's name holds the directory the compiler ran in and,
//! for another crate's file, where that crate's source sits on the machine. The same source
//! checked out somewhere else then names every expansion differently. This names an expansion by
//! what rustc hashes apart from those names: its kind and edition, the macro's definition, the
//! places it was called and defined, and the same for each expansion it sits in. A place is a
//! byte range in a file named by its path from the document root or, for another crate's file,
//! by its content's hash.
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::ty::TyCtxt;
use rustc_span::{ExpnId, Span};
use std::path::Path;

pub(crate) fn identity(tcx: TyCtxt<'_>, root: &Path, expansion: ExpnId) -> String {
    let mut parts = Vec::new();
    let mut current = expansion;
    while current != ExpnId::root() {
        let data = current.expn_data();
        parts.push(format!(
            "{:?} {:?} {} {} {}",
            data.kind,
            data.edition,
            definition(tcx, data.macro_def_id),
            place(tcx, root, data.call_site),
            place(tcx, root, data.def_site),
        ));
        current = data.parent;
    }
    let digest = rstyle_core::cpg::store::sha256(parts.join("\n").as_bytes());
    format!("expansion:{}", &digest[..32])
}

fn definition(tcx: TyCtxt<'_>, definition: Option<DefId>) -> String {
    definition.map_or_else(
        || "-".to_owned(),
        |id| format!("{:?}", tcx.def_path_hash(id)),
    )
}

fn place(tcx: TyCtxt<'_>, root: &Path, span: Span) -> String {
    if span.is_dummy() {
        return "-".to_owned();
    }
    let source_map = tcx.sess.source_map();
    let (lo, hi) = (
        source_map.lookup_byte_offset(span.lo()),
        source_map.lookup_byte_offset(span.hi()),
    );
    let file = match lo.sf.cnum == LOCAL_CRATE {
        true => crate::paths::document_path(tcx, &lo.sf, root),
        false => None,
    }
    .unwrap_or_else(|| format!("{:?}", lo.sf.src_hash));
    format!("{file}:{}..{}", lo.pos.0, hi.pos.0)
}
