//! The concrete grammar's library value carriers: core nonzero integers, ranges of
//! `usize`, and the Result shapes of integer, slice-to-array and fresh conversions.

use super::{Capture, Failure, unsupported};
use crate::concrete_types::{MAX_FIELDS, TypeTree};
use crate::extraction_profile::ExtractionProfile;
use rstyle_core::cpg::model::{Definition, Scope, Type, TypeKind};

impl<'a> Capture<'_, '_, 'a> {
    /// A library carrier the grammar observes before Options and Results.
    pub(super) fn carrier(
        &mut self,
        ty: &'a Type,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if let Some(nonzero) = self.nonzero(ty)? {
            return Ok(Some(nonzero));
        }
        self.range(ty, depth)
    }

    pub(super) fn nonzero(&self, ty: &'a Type) -> Result<Option<TypeTree>, Failure> {
        use crate::nonzero_values::{Definition as Identity, Observation, Scalar};
        if self.context.reader.profile != ExtractionProfile::DataV3 {
            return Ok(None);
        }
        let Some(arguments) = self.standard_arguments(ty, "rust:NonZero") else {
            return Ok(None);
        };
        let [argument] = arguments else {
            return Err(unsupported(
                "NonZero requires one exact concrete integer argument",
            ));
        };
        let scalar = match self.ty(argument)?.spelling.as_str() {
            "u8" => Scalar::U8,
            "u16" => Scalar::U16,
            _ => return Err(unsupported("NonZero supports only concrete u8/u16 values")),
        };
        let definition = self.definition_of(ty)?;
        let identity = |key: &str| self.identity(definition, key);
        let observation = Observation {
            definition: Identity {
                path: definition.name.clone(),
                definition_identity: identity("def_path_hash")?,
                crate_name: identity("crate_name")?,
                stable_crate_id: identity("stable_crate_id")?,
                local: definition.scope == Scope::Unit,
            },
            scalar,
        };
        let observation = observation.canonicalized().map_err(Failure::Unsupported)?;
        Ok(Some(TypeTree::NonZero(observation)))
    }

    /// The standard range type `ty` is, among those the profile observes.
    fn range_kind(&self, ty: &'a Type) -> Option<&'static str> {
        [
            ("RangeTo", "rust.checked-borrowed-result-flow"),
            ("RangeFrom", "rust.checked-suffix-result-views"),
            ("Range", "rust.fresh-range-results"),
        ]
        .into_iter()
        .find(|(name, capability)| {
            self.standard_arguments(ty, &format!("rust:{name}"))
                .is_some()
                && self.supports(capability)
        })
        .map(|(name, _)| name)
    }

    fn range(&self, ty: &'a Type, depth: usize) -> Result<Option<TypeTree>, Failure> {
        use crate::range_values::authority::Observation;
        let Some(kind) = self.range_kind(ty) else {
            return Ok(None);
        };
        let arguments = self
            .standard_arguments(ty, &format!("rust:{kind}"))
            .unwrap_or_default();
        let usize = matches!(arguments, [argument] if self.ty(argument).is_ok_and(|ty| ty.spelling == "usize"));
        if depth != 0 || !usize {
            return Err(unsupported(
                "Range capture requires a root Range<usize> value",
            ));
        }
        let definition = self.definition_of(ty)?;
        let fields = self.range_fields(ty)?;
        let has = |modifier: &str| ty.modifiers.iter().any(|m| m == modifier);
        let observation = Observation {
            definition: self.callee(definition)?,
            lang_item: kind.into(),
            arguments: vec!["usize".into()],
            fields,
            is_copy: has("rust:copy"),
            needs_drop: has("rust:needs-drop"),
            has_drop_impl: definition.destructor,
        };
        Ok(Some(match kind {
            "RangeTo" => TypeTree::RangeToUsize { observation },
            "RangeFrom" => TypeTree::RangeFromUsize { observation },
            _ => TypeTree::RangeUsize { observation },
        }))
    }

    fn range_fields(
        &self,
        ty: &Type,
    ) -> Result<Vec<crate::range_values::authority::Field>, Failure> {
        let fields = ty.fields.as_deref().unwrap_or_default();
        fields
            .iter()
            .enumerate()
            .map(|(position, field)| {
                Ok(crate::range_values::authority::Field {
                    position,
                    name: field.name.clone(),
                    scalar: self.ty(&field.ty)?.spelling.clone(),
                })
            })
            .collect()
    }

    /// A Result shape the grammar observes: integer, slice-to-array or fresh conversions.
    pub(super) fn observed_result(
        &mut self,
        ty: &'a Type,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        let Some((ok, err)) = self.result_payloads(ty) else {
            return Ok(None);
        };
        if let Some(result) = self.never_conversion(ok, err, depth)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.mapped_conversion(ok, err, depth)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.int_conversion(ok, err)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.slice_array(ok, err)? {
            return Ok(Some(result));
        }
        if self.supports("rust.fresh-concrete-results") && self.visiting.is_empty() {
            let result = TypeTree::Result {
                ok: Box::new(self.tree(ok, depth + 1)?),
                err: Box::new(self.tree(err, depth + 1)?),
            };
            crate::concrete_types::validate(&result).map_err(Failure::Unsupported)?;
            return Ok(Some(result));
        }
        Ok(None)
    }

    fn never_conversion(
        &mut self,
        ok: &'a str,
        err: &'a str,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if !self.supports("rust.u16-usize-reader-observations")
            || self.ty(err)?.kind != TypeKind::Bottom
        {
            return Ok(None);
        }
        if !(self.spelled(ok, "usize") || self.usize_pair(ok)) {
            return Ok(None);
        }
        let result = TypeTree::NeverConversionResult {
            ok: Box::new(self.tree(ok, depth + 1)?),
        };
        crate::concrete_types::validate(&result).map_err(Failure::Unsupported)?;
        Ok(Some(result))
    }

    fn mapped_conversion(
        &mut self,
        ok: &'a str,
        err: &'a str,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if !self.supports("rust.usize-reader-callback-observations")
            || !self.usize_pair(ok)
            || !self.int_conversion_error(err)
        {
            return Ok(None);
        }
        let result = TypeTree::MappedIntConversionResult {
            ok: Box::new(self.tree(ok, depth + 1)?),
        };
        crate::concrete_types::validate(&result).map_err(Failure::Unsupported)?;
        Ok(Some(result))
    }

    fn int_conversion(&self, ok: &'a str, err: &'a str) -> Result<Option<TypeTree>, Failure> {
        if !self.supports("rust.bitset-cardinality") {
            return Ok(None);
        }
        let target = self.ty(ok)?.spelling.as_str();
        let admitted = target == "usize"
            || self.supports("rust.end-of-input") && target == "u16"
            || self.supports("rust.lossless-usize-u64-calls") && target == "u64";
        Ok((admitted && self.int_conversion_error(err))
            .then(|| TypeTree::IntConversionResult(target.into())))
    }

    fn slice_array(&self, ok: &'a str, err: &'a str) -> Result<Option<TypeTree>, Failure> {
        if !self.supports("rust.slice-array-byte-calls") {
            return Ok(None);
        }
        let error = self.ty(err)?;
        let slice_error = self
            .core_definition(error)
            .is_some_and(|definition| definition.name.ends_with("::array::TryFromSliceError"));
        if !slice_error {
            return Ok(None);
        }
        let ok = self.ty(ok)?;
        let (array, borrowed) = match (ok.kind, ok.mode.as_deref(), ok.target.as_deref()) {
            (TypeKind::Reference, Some("shared"), Some(target)) => (self.ty(target)?, true),
            _ => (ok, false),
        };
        let bytes = array.kind == TypeKind::Sequence
            && array.fixed == Some(true)
            && array
                .element
                .as_deref()
                .is_some_and(|element| self.spelled(element, "u8"));
        Ok(match (bytes, array.length) {
            (true, Some(length)) if length <= MAX_FIELDS as u64 => {
                Some(TypeTree::SliceArrayResult { length, borrowed })
            }
            _ => None,
        })
    }

    fn int_conversion_error(&self, err: &str) -> bool {
        self.ty(err).ok().is_some_and(|error| {
            let empty = error.arguments.as_ref().is_none_or(Vec::is_empty);
            empty
                && self.core_definition(error).is_some_and(|definition| {
                    matches!(
                        definition.name.as_str(),
                        "core::num::error::TryFromIntError"
                            | "core::num::TryFromIntError"
                            | "std::num::TryFromIntError"
                    )
                })
        })
    }

    /// A nominal type's definition when the `core` crate defines it.
    fn core_definition(&self, ty: &Type) -> Option<&'a Definition> {
        if !matches!(ty.kind, TypeKind::Record | TypeKind::Sum) {
            return None;
        }
        let definition = self.definition_of(ty).ok()?;
        (self.identity(definition, "crate_name").ok()? == "core").then_some(definition)
    }

    fn spelled(&self, id: &str, spelling: &str) -> bool {
        self.ty(id).is_ok_and(|ty| ty.spelling == spelling)
    }

    fn usize_pair(&self, id: &str) -> bool {
        self.ty(id).is_ok_and(|ty| {
            matches!(ty.items.as_deref(), Some([first, second])
                if ty.kind == TypeKind::Tuple && self.spelled(first, "usize") && self.spelled(second, "usize"))
        })
    }

    /// A definition as the frontend records a library callee.
    fn callee(&self, definition: &Definition) -> Result<crate::data_library::Callee, Failure> {
        Ok(crate::data_library::Callee {
            path: definition.name.clone(),
            crate_name: self.identity(definition, "crate_name")?,
            crate_hash: self
                .identity(definition, "crate_hash")
                .unwrap_or_else(|_| "local-unencoded".into()),
            definition_hash: self.identity(definition, "def_path_hash")?,
            implementation: None,
        })
    }

    fn identity(&self, definition: &Definition, key: &str) -> Result<String, Failure> {
        definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get(key)?.as_str())
            .map(str::to_owned)
            .ok_or_else(|| unsupported("Missing compiler identity"))
    }
}
