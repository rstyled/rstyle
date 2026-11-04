//! Channel-by-channel differences between the driver's facts and those read from its
//! document. Frontend options are compared per family, not here.

use crate::frontend::{HirFnFacts, HirNode, ParamFact, SignatureFacts};
use serde::Serialize;
use std::fmt::Debug;

/// One channel whose reference and derived values differ.
#[derive(Debug, Serialize)]
pub struct Difference {
    /// The inventory channel, such as `node.kind` or `signature.return_ty`.
    pub channel: String,
    /// Where in the channel: a body node's child path, or a parameter's position.
    pub path: String,
    pub reference: String,
    pub derived: String,
}

// rstyle-allow(mutation_per_target): collects every channel difference into one list
pub fn differences(reference: &HirFnFacts, derived: &HirFnFacts) -> Vec<Difference> {
    let mut found = Differences::default();
    found.field(("function.name", ""), &reference.name, &derived.name);
    found.field(
        ("function.fn_span", ""),
        &reference.fn_span,
        &derived.fn_span,
    );
    found.field(
        ("function.source", ""),
        reference.source.bytes.as_str(),
        derived.source.bytes.as_str(),
    );
    found.field(
        ("function.digest", ""),
        &reference.source.digest,
        &derived.source.digest,
    );
    found.signature(&reference.signature, &derived.signature);
    found.list("function.bindings", &reference.bindings, &derived.bindings);
    found.node("", &reference.body, &derived.body);
    found.field(
        ("build", ""),
        &reference.build.settings(),
        &derived.build.settings(),
    );
    found.list(
        "function.source_diagnostics",
        &reference.source_diagnostics,
        &derived.source_diagnostics,
    );
    found.options(reference, derived);
    found.field(
        ("function.source_scan_complete", ""),
        &reference.source_scan_complete,
        &derived.source_scan_complete,
    );

    found.0
}

/// A channel and the place within it.
type Site<'s> = (&'s str, &'s str);

#[derive(Default)]
struct Differences(Vec<Difference>);

impl Differences {
    fn field<T: PartialEq + Debug + ?Sized>(&mut self, site: Site<'_>, reference: &T, derived: &T) {
        if reference != derived {
            self.push(site, format!("{reference:?}"), format!("{derived:?}"));
        }
    }

    fn push(&mut self, (channel, path): Site<'_>, reference: String, derived: String) {
        self.0.push(Difference {
            channel: channel.into(),
            path: path.into(),
            reference,
            derived,
        });
    }

    fn list<T: PartialEq + Debug>(&mut self, channel: &str, reference: &[T], derived: &[T]) {
        if reference.len() != derived.len() {
            self.field((channel, "len"), &reference.len(), &derived.len());
            return;
        }
        for (index, (reference, derived)) in reference.iter().zip(derived).enumerate() {
            self.field((channel, &index.to_string()), reference, derived);
        }
    }

    /// The frontend option families the reader rebuilds, each as its own channel.
    fn options(&mut self, reference: &HirFnFacts, derived: &HirFnFacts) {
        use crate::concrete_types::InterfaceObservations;
        let (reference, derived) = (
            &reference.build.frontend_options,
            &derived.build.frontend_options,
        );
        let observed = |options: &crate::frontend::FrontendOptions| {
            options
                .data::<InterfaceObservations>()
                .ok()
                .map(|data| data.into_owned())
        };
        self.field(
            ("option.concrete-types", ""),
            &observed(reference),
            &observed(derived),
        );
        // Every other option family, each by its key, as the frontend records it.
        let keys: std::collections::BTreeSet<&str> =
            reference.keys().chain(derived.keys()).collect();

        for key in keys {
            if key == crate::concrete_types::KEY {
                continue;
            }
            let channel = format!("option.{key}");
            self.field((&channel, ""), &reference.get(key), &derived.get(key));
        }
    }

    // rstyle-allow(mutation_per_target): collects every channel difference into one list
    fn signature(&mut self, reference: &SignatureFacts, derived: &SignatureFacts) {
        self.field(
            ("signature.sig_span", ""),
            &reference.sig_span,
            &derived.sig_span,
        );
        self.field(
            ("signature.receiver", ""),
            &reference.receiver,
            &derived.receiver,
        );
        self.field(
            ("signature.return_ty", ""),
            &reference.return_ty,
            &derived.return_ty,
        );
        self.field(
            ("signature.return_ty_span", ""),
            &reference.return_ty_span,
            &derived.return_ty_span,
        );
        self.field(("signature.abi", ""), &reference.abi, &derived.abi);
        self.field(
            ("signature.generics", ""),
            &reference.generics,
            &derived.generics,
        );
        self.field(
            ("signature.modifiers", ""),
            &modifiers(reference),
            &modifiers(derived),
        );
        if reference.params.len() != derived.params.len() {
            let site = ("signature.params", "len");
            self.field(site, &reference.params.len(), &derived.params.len());
            return;
        }

        for (reference, derived) in reference.params.iter().zip(&derived.params) {
            self.parameter(reference, derived);
        }
    }

    // rstyle-allow(mutation_per_target): collects every channel difference into one list
    fn parameter(&mut self, reference: &ParamFact, derived: &ParamFact) {
        let path = reference.position.to_string();
        self.field(
            ("signature.param.symbol", &path),
            &reference.symbol,
            &derived.symbol,
        );
        let site = ("signature.param.name_span", path.as_str());
        self.field(site, &reference.name_span, &derived.name_span);
        let site = ("signature.param.ty_span", path.as_str());
        self.field(site, &reference.ty_span, &derived.ty_span);
        self.field(("signature.param.ty", &path), &reference.ty, &derived.ty);

        self.node(&format!("param{path}"), &reference.pat, &derived.pat);
    }

    fn node(&mut self, path: &str, reference: &HirNode, derived: &HirNode) {
        self.node_fields(path, reference, derived);
        if reference.children.len() != derived.children.len() {
            let (reference, derived) = (kinds(&reference.children), kinds(&derived.children));
            self.push(("node.children", path), reference, derived);
            return;
        }
        let children = reference.children.iter().zip(&derived.children);
        for (index, (reference, derived)) in children.enumerate() {
            self.node(&format!("{path}/{index}"), reference, derived);
        }
    }

    // rstyle-allow(mutation_per_target): collects every channel difference into one list
    fn node_fields(&mut self, path: &str, reference: &HirNode, derived: &HirNode) {
        self.field(("node.hir", path), &reference.hir, &derived.hir);
        self.field(("node.kind", path), &reference.kind, &derived.kind);
        self.field(("node.span", path), &reference.span, &derived.span);
        self.field(("node.snippet", path), &reference.snippet, &derived.snippet);
        self.field(("node.ty", path), &reference.ty, &derived.ty);
        self.field(("node.res", path), &reference.res, &derived.res);
        self.field(
            ("node.dispatch", path),
            &reference.dispatch,
            &derived.dispatch,
        );
        self.field(
            ("node.expansion", path),
            &reference.expansion,
            &derived.expansion,
        );
        self.field(("node.binding", path), &reference.binding, &derived.binding);
        self.field(
            ("node.binding_mut", path),
            &reference.binding_mut,
            &derived.binding_mut,
        );
        self.field(("node.lit", path), &reference.lit, &derived.lit);
        self.field(
            ("node.param_index", path),
            &reference.param_index,
            &derived.param_index,
        );
        let unsafe_block = (&reference.is_unsafe_block, &derived.is_unsafe_block);

        self.field(
            ("node.is_unsafe_block", path),
            unsafe_block.0,
            unsafe_block.1,
        );
    }
}

fn modifiers(signature: &SignatureFacts) -> [bool; 5] {
    [
        signature.has_self,
        signature.is_variadic,
        signature.is_async,
        signature.is_unsafe,
        signature.is_const,
    ]
}

fn kinds(nodes: &[HirNode]) -> String {
    let kinds: Vec<&str> = nodes.iter().map(|node| node.kind.as_str()).collect();
    kinds.join(",")
}
