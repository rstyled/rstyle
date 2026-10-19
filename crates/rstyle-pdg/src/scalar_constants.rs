//! Resolved scalar constant values supplied by rustc, without runtime reads.

use crate::{HirNode, PathRes};
use serde::{Deserialize, Serialize};
use syn::ext::IdentExt;

pub const RESOLUTION: &str = "resolved_scalar_constant";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarConstant {
    pub definition: String,
    pub declaration: String,
    pub ty: String,
    pub value: String,
}

impl ScalarConstant {
    pub fn decode(resolution: &PathRes) -> Result<Self, String> {
        if resolution.kind != RESOLUTION || resolution.target.is_some() {
            return Err("Expected a resolved scalar constant".into());
        }
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())
    }

    pub fn verify(node: &HirNode, pointer_width: Option<u64>) -> Result<Self, String> {
        if node.kind != "expr_path" {
            return Err("Expected a constant expression path".into());
        }
        Self::verify_value(node, pointer_width)
    }

    pub fn verify_pattern(node: &HirNode, pointer_width: Option<u64>) -> Result<Self, String> {
        let constant = Self::verify_value(node, pointer_width)?;
        if node.kind != "pat_constant" || node.lit.as_deref() != Some(&constant.value) {
            return Err("Constant pattern selector disagrees with its resolved value".into());
        }
        Ok(constant)
    }

    fn verify_value(node: &HirNode, pointer_width: Option<u64>) -> Result<Self, String> {
        let resolution = node.res.as_ref().ok_or("Missing constant resolution")?;
        let constant = Self::decode(resolution)?;
        if !node.children.is_empty()
            || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(&constant.ty)
            || !constant.value_valid(pointer_width)
        {
            return Err("Constant type, value or expression shape disagrees".into());
        }

        constant.check_declaration()?;
        Ok(constant)
    }

    fn check_declaration(&self) -> Result<(), String> {
        let declaration: syn::ItemConst = syn::parse_str(&self.declaration)
            .map_err(|error| format!("Missing original constant declaration: {error}"))?;
        let name = declaration.ident.unraw().to_string();
        if self.definition.rsplit("::").next() != Some(&name)
            || !declaration.generics.params.is_empty()
            || primitive_type(&declaration.ty).as_deref() != Some(&self.ty)
        {
            return Err("Resolved constant definition or declared type disagrees".into());
        }
        Ok(())
    }

    pub fn value_valid(&self, pointer_width: Option<u64>) -> bool {
        match self.ty.as_str() {
            "bool" => matches!(self.value.as_str(), "true" | "false"),
            "unit" => self.value == "()",
            ty => {
                crate::concrete_types::scalar(ty)
                    && integer_width(ty, pointer_width)
                        .is_some_and(|width| integer_value(&self.value, ty.starts_with('i'), width))
            }
        }
    }
}

fn primitive_type(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Tuple(tuple) if tuple.elems.is_empty() => Some("unit".into()),
        syn::Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            let segment = &path.path.segments[0];
            matches!(segment.arguments, syn::PathArguments::None)
                .then(|| segment.ident.unraw().to_string())
        }
        _ => None,
    }
}

fn integer_width(ty: &str, pointer_width: Option<u64>) -> Option<u64> {
    let width = match ty {
        "isize" | "usize" => pointer_width?,
        _ => ty.strip_prefix(['i', 'u'])?.parse().ok()?,
    };
    matches!(width, 8 | 16 | 32 | 64 | 128).then_some(width)
}

fn integer_value(text: &str, signed: bool, width: u64) -> bool {
    if signed {
        return text.parse::<i128>().is_ok_and(|value| {
            value.to_string() == text
                && (width == 128
                    || (-(1i128 << (width - 1))..(1i128 << (width - 1))).contains(&value))
        });
    }

    text.parse::<u128>()
        .is_ok_and(|value| value.to_string() == text && (width == 128 || value < 1u128 << width))
}
