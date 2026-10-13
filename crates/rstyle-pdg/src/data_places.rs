//! Shared field-place ownership derived from original source and resolved locations.

use crate::{
    HirFnFacts, HirNode,
    data_origins::Origin,
    data_types::{self, TypeToken},
    field_locations::{self, Location, Projection},
};
use std::collections::BTreeSet;

pub const KEY: &str = "rstyle.data:place-location";
type Key = (u32, u32, u32);

pub struct Places {
    fields: BTreeSet<Key>,
}

impl Places {
    pub fn verify(facts: &HirFnFacts) -> Result<Self, String> {
        let mut places = Self {
            fields: BTreeSet::new(),
        };
        places.visit(&facts.body, facts)?;
        Ok(places)
    }

    pub fn contains(&self, node: &HirNode) -> bool {
        self.fields.contains(&key(node))
    }

    fn visit(&mut self, node: &HirNode, facts: &HirFnFacts) -> Result<(), String> {
        if node.kind == "expr_addr_of" {
            borrow_origin(node, facts)?;
            self.insert_fields(&node.children[0])?;
        }
        for child in &node.children {
            self.visit(child, facts)?;
        }
        Ok(())
    }

    fn insert_fields(&mut self, node: &HirNode) -> Result<(), String> {
        match node.kind.as_str() {
            "expr_field" => {
                self.fields.insert(key(node));
                let [base] = node.children.as_slice() else {
                    return Err("Place lacks one exact base".into());
                };
                self.insert_fields(base)
            }
            "expr_path" => Ok(()),
            _ => Err("Borrow place requires a formal field path".into()),
        }
    }
}

fn key(node: &HirNode) -> Key {
    (node.hir.krate, node.hir.owner, node.hir.local)
}

pub fn borrow_origin(node: &HirNode, facts: &HirFnFacts) -> Result<Origin, String> {
    let original = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original borrow range")?;
    let syn::Expr::Reference(source) = syn::parse_str(original).map_err(|e| e.to_string())? else {
        return Err("Borrow lacks original shared-reference syntax".into());
    };
    if source.mutability.is_some() || !matches!(source.expr.as_ref(), syn::Expr::Field(_)) {
        return Err("Borrow requires an original shared formal-field place".into());
    }

    let [place] = node.children.as_slice() else {
        return Err("Borrow lacks one exact place operand".into());
    };
    if place.kind != "expr_field" {
        return Err("Unsupported borrow place".into());
    }
    let fields: field_locations::FieldObservations =
        crate::data_frontend::evidence(facts, field_locations::KEY)?;
    let location = fields
        .operations
        .iter()
        .find(|field| field.hir == place.hir)
        .ok_or("Missing borrowed field location")?;
    let Location::Captured {
        parameter,
        path,
        value,
    } = &location.location
    else {
        return Err("Unproved borrow place".into());
    };
    let referent = TypeToken::from_tree(value);
    if crate::data_origins::shared(&referent) || !path.contains(&Projection::SharedReferent) {
        return Err("Borrow target requires a non-reference-bearing shared formal referent".into());
    }

    let expected = TypeToken::Shared(Box::new(referent)).text()?;
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(&expected)
        || data_types::parse(&expected).is_err()
    {
        return Err("Borrow result/referent type correspondence disagrees".into());
    }
    Ok(Origin {
        parameter: *parameter,
        path: path.clone(),
    })
}
