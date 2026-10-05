//! Signature normalization: `Self` and positional type variables, so that
//! `Dog::bark(&self) -> String` and `Cat::meow(&self) -> String` share one key.
//!
//! Policy lives here, not in the driver: facts carry raw `TyShape`s.

use crate::facts::{Receiver, SigFacts, TyShape};

/// Crates whose ADTs stay nominal (never abstracted into a type variable).
/// Every other ADT is a candidate for alpha-renaming.
const NOMINAL_CRATES: [&str; 3] = ["core", "alloc", "std"];

/// A signature with `Self` resolved and other ADTs alpha-renamed in order of first use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormSig {
    /// Canonical rendering, e.g. `fn(&Self, &A, &A) -> Self`. Equal keys, equal signatures.
    pub key: String,
    /// Distinct non-`Self`, non-std ADT variables.
    pub vars: usize,
}

/// Last path segment: `alloc::string::String` -> `String`.
pub(crate) fn short(id: &str) -> &str {
    id.rsplit("::").next().unwrap_or(id)
}

fn is_nominal(id: &str) -> bool {
    id.split("::")
        .next()
        .is_some_and(|krate| NOMINAL_CRATES.contains(&krate))
}

fn var_name(index: usize) -> String {
    let letter = char::from(b'A' + (index % 26) as u8);
    match index / 26 {
        0 => letter.to_string(),
        round => format!("{letter}{round}"),
    }
}

/// Children that `render` descends into (`Self` hides its own arguments).
fn children<'a>(ty: &'a TyShape, self_ty: Option<&str>) -> Vec<&'a TyShape> {
    match ty {
        TyShape::Adt { id, .. } if self_ty == Some(id.as_str()) => vec![],
        TyShape::Adt { args, .. } => args.iter().collect(),
        TyShape::Tuple(items) => items.iter().collect(),
        TyShape::Ref { inner, .. } | TyShape::Slice(inner) | TyShape::Array(inner) => {
            vec![inner]
        }
        _ => vec![],
    }
}

/// ADT ids that become type variables, in render order (pre-order, with repeats).
fn variable_ids<'a>(ty: &'a TyShape, self_ty: Option<&str>) -> Vec<&'a str> {
    let own = match ty {
        TyShape::Adt { id, .. } if self_ty != Some(id.as_str()) && !is_nominal(id) => {
            Some(id.as_str())
        }
        _ => None,
    };
    let below = children(ty, self_ty)
        .into_iter()
        .flat_map(|child| variable_ids(child, self_ty));
    own.into_iter().chain(below).collect()
}

/// Rendering context: `Self` and the variable numbering are fixed up front,
/// so rendering itself is pure.
struct Namer<'a> {
    self_ty: Option<&'a str>,
    /// Distinct variable ADT ids in order of first use; the position is the variable.
    vars: Vec<&'a str>,
}

impl<'a> Namer<'a> {
    fn new(self_ty: Option<&'a str>, types: &[&'a TyShape]) -> Self {
        let mut vars: Vec<&str> = Vec::new();
        for id in types.iter().flat_map(|ty| variable_ids(ty, self_ty)) {
            if !vars.contains(&id) {
                vars.push(id);
            }
        }
        Namer { self_ty, vars }
    }

    fn list(&self, items: &[TyShape]) -> String {
        let parts: Vec<String> = items.iter().map(|item| self.render(item)).collect();
        parts.join(", ")
    }

    fn adt(&self, id: &str, args: &[TyShape]) -> String {
        if self.self_ty == Some(id) {
            return "Self".into(); // `Self` already fixes its own arguments
        }
        let base = if is_nominal(id) {
            short(id).to_string()
        } else {
            let index = self.vars.iter().position(|seen| *seen == id);
            var_name(index.unwrap_or(0))
        };
        if args.is_empty() {
            base
        } else {
            format!("{base}<{}>", self.list(args))
        }
    }

    fn render(&self, ty: &TyShape) -> String {
        match ty {
            TyShape::Adt { id, args } => self.adt(id, args),
            TyShape::Ref { mutable, inner } => {
                let prefix = if *mutable { "&mut " } else { "&" };
                format!("{prefix}{}", self.render(inner))
            }
            TyShape::Tuple(items) => format!("({})", self.list(items)),
            TyShape::Slice(inner) => format!("[{}]", self.render(inner)),
            TyShape::Array(inner) => format!("[{}; _]", self.render(inner)),
            TyShape::Prim(name) => name.clone(),
            TyShape::Param(index) => format!("T{index}"),
            TyShape::FnPtr => "fn".into(),
            TyShape::Dyn(id) => format!("dyn {}", short(id)),
            TyShape::Opaque => "impl _".into(),
            TyShape::Other => "_".into(),
        }
    }
}

fn receiver_text(receiver: Receiver) -> Option<&'static str> {
    match receiver {
        Receiver::None => None,
        Receiver::Value => Some("Self"),
        Receiver::Ref => Some("&Self"),
        Receiver::RefMut => Some("&mut Self"),
        Receiver::Other => Some("self: _"),
    }
}

fn qualifiers(sig: &SigFacts) -> String {
    let flags = [
        (sig.is_const, "const "),
        (sig.is_async, "async "),
        (sig.is_unsafe, "unsafe "),
    ];
    let mut text: String = flags
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, word)| *word)
        .collect();

    text.push_str("fn");
    if sig.generics > 0 {
        text.push_str(&format!("<{}>", sig.generics));
    }
    text
}

pub fn normalize(sig: &SigFacts) -> NormSig {
    let types: Vec<&TyShape> = sig.params.iter().chain([&sig.ret]).collect();
    let namer = Namer::new(sig.self_ty.as_deref(), &types);
    let mut args: Vec<String> = receiver_text(sig.receiver)
        .map(String::from)
        .into_iter()
        .collect();

    args.extend(sig.params.iter().map(|ty| namer.render(ty)));
    NormSig {
        key: format!(
            "{}({}) -> {}",
            qualifiers(sig),
            args.join(", "),
            namer.render(&sig.ret)
        ),
        vars: namer.vars.len(),
    }
}

/// Coarse class of a type for graph labels: std ADTs keep their name, every
/// other ADT is `_`, so `&[Dog]` and `&[Cat]` share a class.
pub fn class_of(ty: &TyShape) -> String {
    let list = |items: &[TyShape]| items.iter().map(class_of).collect::<Vec<_>>().join(", ");
    match ty {
        TyShape::Adt { id, args } if is_nominal(id) && args.is_empty() => short(id).to_string(),
        TyShape::Adt { id, args } if is_nominal(id) => format!("{}<{}>", short(id), list(args)),
        TyShape::Adt { .. } => "_".into(),
        TyShape::Ref { mutable, inner } => {
            format!("&{}{}", if *mutable { "mut " } else { "" }, class_of(inner))
        }
        TyShape::Tuple(items) => format!("({})", list(items)),
        TyShape::Slice(inner) => format!("[{}]", class_of(inner)),
        TyShape::Array(inner) => format!("[{}; _]", class_of(inner)),
        TyShape::Prim(name) => name.clone(),
        TyShape::Param(index) => format!("T{index}"),
        TyShape::Dyn(id) => format!("dyn {}", short(id)),
        TyShape::FnPtr | TyShape::Opaque | TyShape::Other => "_".into(),
    }
}

fn zip_children<'a>(a: &'a TyShape, b: &'a TyShape) -> Vec<(&'a TyShape, &'a TyShape)> {
    match (a, b) {
        (TyShape::Adt { args: x, .. }, TyShape::Adt { args: y, .. }) => x.iter().zip(y).collect(),
        (TyShape::Tuple(x), TyShape::Tuple(y)) => x.iter().zip(y).collect(),
        (TyShape::Ref { inner: x, .. }, TyShape::Ref { inner: y, .. })
        | (TyShape::Slice(x), TyShape::Slice(y))
        | (TyShape::Array(x), TyShape::Array(y)) => vec![(&**x, &**y)],
        _ => vec![],
    }
}

/// Pairs of differing ADT ids at the same position of two otherwise parallel types
/// (`&[Dog]` vs `&[Cat]` gives `(Dog, Cat)`), in pre-order.
pub fn adt_diffs(a: &TyShape, b: &TyShape) -> Vec<(String, String)> {
    match (a, b) {
        (TyShape::Adt { id: x, .. }, TyShape::Adt { id: y, .. }) if x != y => {
            vec![(x.clone(), y.clone())]
        }
        _ => zip_children(a, b)
            .into_iter()
            .flat_map(|(x, y)| adt_diffs(x, y))
            .collect(),
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn adt(id: &str) -> TyShape {
        TyShape::Adt {
            id: id.into(),
            args: vec![],
        }
    }

    pub fn string() -> TyShape {
        adt("alloc::string::String")
    }

    pub fn by_ref(inner: TyShape) -> TyShape {
        TyShape::Ref {
            mutable: false,
            inner: Box::new(inner),
        }
    }

    pub fn method(
        self_ty: &str,
        receiver: Receiver,
        params: Vec<TyShape>,
        ret: TyShape,
    ) -> SigFacts {
        SigFacts {
            self_ty: Some(self_ty.into()),
            self_args: vec![],
            receiver,
            params,
            ret,
            is_async: false,
            is_unsafe: false,
            is_const: false,
            generics: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn key(sig: &SigFacts) -> String {
        normalize(sig).key
    }

    #[test]
    fn bark_and_meow_normalize_equal() {
        let bark = method("animals::Dog", Receiver::Ref, vec![], string());
        let meow = method("animals::Cat", Receiver::Ref, vec![], string());
        assert_eq!(normalize(&bark), normalize(&meow));
        assert_eq!(key(&bark), "fn(&Self) -> String");
    }

    #[test]
    fn ref_mut_owned_receivers_stay_distinct() {
        let keys: Vec<String> = [
            Receiver::Ref,
            Receiver::RefMut,
            Receiver::Value,
            Receiver::None,
        ]
        .into_iter()
        .map(|r| key(&method("k::Dog", r, vec![], string())))
        .collect();
        let mut unique = keys.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 4, "{keys:?}");
    }

    #[test]
    fn other_adt_params_get_consistent_positional_vars() {
        let (a, b) = (adt("k::Cat"), adt("k::Rock"));
        let same = method(
            "k::Dog",
            Receiver::Ref,
            vec![by_ref(a.clone()), by_ref(a.clone())],
            string(),
        );
        let diff = method(
            "k::Dog",
            Receiver::Ref,
            vec![by_ref(a.clone()), by_ref(b.clone())],
            string(),
        );
        assert_eq!(key(&same), "fn(&Self, &A, &A) -> String");
        assert_eq!(key(&diff), "fn(&Self, &A, &B) -> String");
        let renamed = method(
            "k::Pup",
            Receiver::Ref,
            vec![by_ref(b.clone()), by_ref(b)],
            string(),
        );
        assert_eq!(key(&same), key(&renamed));
        assert_eq!(normalize(&diff).vars, 2);
    }

    #[test]
    fn std_types_stay_nominal() {
        let vec_u8 = TyShape::Adt {
            id: "alloc::vec::Vec".into(),
            args: vec![TyShape::Prim("u8".into())],
        };
        let vec_dog = TyShape::Adt {
            id: "alloc::vec::Vec".into(),
            args: vec![adt("k::Dog")],
        };
        let a = method("k::Dog", Receiver::Ref, vec![], vec_u8);
        let b = method("k::Dog", Receiver::Ref, vec![], string());
        let c = method("k::Cat", Receiver::Ref, vec![], vec_dog);
        assert_eq!(key(&a), "fn(&Self) -> Vec<u8>");
        assert_ne!(key(&a), key(&b));
        assert_eq!(key(&c), "fn(&Self) -> Vec<A>");
    }

    #[test]
    fn class_hides_workspace_adts_and_keeps_std() {
        let dogs = TyShape::Slice(Box::new(adt("k::Dog")));
        let cats = TyShape::Slice(Box::new(adt("k::Cat")));
        assert_eq!(class_of(&by_ref(dogs.clone())), "&[_]");
        assert_eq!(
            class_of(&by_ref(dogs.clone())),
            class_of(&by_ref(cats.clone()))
        );
        assert_eq!(class_of(&string()), "String");
        assert_eq!(
            adt_diffs(&by_ref(dogs), &by_ref(cats)),
            [("k::Dog".to_string(), "k::Cat".to_string())]
        );
        assert!(adt_diffs(&string(), &string()).is_empty());
    }

    #[test]
    fn self_in_return_position() {
        let dog = method(
            "k::Dog",
            Receiver::Ref,
            vec![by_ref(adt("k::Cat"))],
            adt("k::Dog"),
        );
        let cat = method(
            "k::Cat",
            Receiver::Ref,
            vec![by_ref(adt("k::Dog"))],
            adt("k::Cat"),
        );
        assert_eq!(key(&dog), "fn(&Self, &A) -> Self");
        assert_eq!(key(&dog), key(&cat));
        let other = method("k::Dog", Receiver::Ref, vec![], adt("k::Cat"));
        assert_eq!(key(&other), "fn(&Self) -> A");
    }
}
