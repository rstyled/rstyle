//! Field place/value context derived from original source and resolved locations.

use crate::{
    HirFnFacts, HirNode,
    data_origins::Origin,
    data_types::{self, TypeToken},
    field_locations::{FieldObservations, Location, Projection},
};
use std::collections::BTreeSet;

pub const KEY: &str = "rstyle.data:place-location";
type Key = (u32, u32, u32);

pub struct Places {
    fields: BTreeSet<Key>,
}

impl Places {
    pub fn verify(facts: &HirFnFacts) -> Result<Self, String> {
        verify_dereferences(facts)?;
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
        } else if node.kind == "expr_field" && !crate::data_value_fields::candidate(node, facts) {
            let [base] = node.children.as_slice() else {
                return Err("Field projection lacks one exact base".into());
            };
            if is_place_base(base)? {
                self.insert_fields(base)?;
            }
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
                if is_place_base(base)? {
                    self.insert_fields(base)
                } else {
                    Ok(())
                }
            }
            "expr_path" => Ok(()),
            _ => Err("Borrow place requires a formal field path".into()),
        }
    }
}

pub fn is_place_base(node: &HirNode) -> Result<bool, String> {
    if node.kind != "expr_field" {
        return Ok(false);
    }
    let token = node
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .ok_or("Missing field base type")?;
    Ok(!matches!(data_types::parse(token)?, TypeToken::Shared(_)))
}

fn verify_dereferences(facts: &HirFnFacts) -> Result<(), String> {
    let fields = facts.build.frontend_options.data::<FieldObservations>()?;
    for field in &fields.operations {
        if let Location::Captured { path, .. } = &field.location
            && path
                .windows(2)
                .any(|steps| steps == [Projection::SharedReferent, Projection::SharedReferent])
        {
            return Err(
                "Consecutive implicit dereferences need unavailable referent-storage read evidence"
                    .into(),
            );
        }
    }
    Ok(())
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
    let expression: syn::Expr = syn::parse_str(original).map_err(|e| e.to_string())?;
    let syn::Expr::Reference(source) = grouped_borrow(&expression) else {
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
    let fields = facts.build.frontend_options.data::<FieldObservations>()?;
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

fn grouped_borrow(mut expression: &syn::Expr) -> &syn::Expr {
    while let syn::Expr::Paren(inner) = expression {
        expression = &inner.expr;
    }
    expression
}
