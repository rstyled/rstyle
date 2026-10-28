//! Density-only classification of retained scalar bodies. No mutation authority is granted.
use super::*;

pub(super) struct Walker<'a, 'u> {
    pub session: &'a effects::Session<'u>,
    pub paths: &'a EffectPaths,
    pub cache: &'a mut Cache,
}

impl Walker<'_, '_> {
    pub(super) fn call_kinds(
        &mut self,
        owner: usize,
        occurrence: usize,
        remaining: u8,
    ) -> Result<Option<Kinds<'_>>, String> {
        let call = &self.session.observations(owner).unwrap().calls[occurrence];
        if remaining == 0 {
            return Ok(None);
        }
        if let Availability::Known(callee) = &call.resolved_callee
            && !callee.local
            && callee.safe
            && matches!(callee.result_type.as_str(), "u32" | "unit" | "!")
            && call.arguments.iter().all(|argument| {
                matches!(
                    argument.value,
                    Availability::Known(mutations::ArgumentValue::OwnedU32)
                )
            })
        {
            return Ok(self.paths.classify(&callee.path).map(|kind| Kinds {
                direct: Some(kind),
                inherited: None,
            }));
        }

        let Some(callee) = self
            .session
            .retained_local_callee(owner, occurrence, remaining - 1)?
        else {
            return Ok(None);
        };
        let key = CacheKey::Retained {
            owner: callee,
            remaining: remaining - 1,
        };
        if !self.cache.contains_key(&key) {
            let kinds = self.body_kinds(callee, remaining - 1)?;
            self.cache.insert(key, kinds);
        }

        Ok(self.cache[&key].as_ref().map(|inherited| Kinds {
            direct: call
                .target_path
                .as_deref()
                .and_then(|path| self.paths.classify(path)),
            inherited: Some(inherited),
        }))
    }

    fn body_kinds(
        &mut self,
        owner: usize,
        remaining: u8,
    ) -> Result<Option<BTreeSet<EffectKind>>, String> {
        let observed = self.session.observations(owner).unwrap();
        if !matches!(observed.signature, Availability::Known(()))
            || !matches!(observed.direct.availability, Availability::Known(()))
            || observed.direct.execution != mutations::Execution::Completed
            || observed.direct.sites.iter().any(|site| site.caller_visible)
            || observed.direct.boundaries.len() != observed.calls.len()
        {
            return Ok(None);
        }

        check_roster(observed)?;
        let mut kinds = BTreeSet::new();
        for occurrence in 0..observed.calls.len() {
            let Some(inner) = self.call_kinds(owner, occurrence, remaining)? else {
                return Ok(None);
            };
            kinds.extend(inner);
        }

        Ok(Some(kinds))
    }
}

pub(super) struct Kinds<'a> {
    direct: Option<EffectKind>,
    inherited: Option<&'a BTreeSet<EffectKind>>,
}

impl<'a> IntoIterator for Kinds<'a> {
    type Item = EffectKind;
    type IntoIter = std::iter::Chain<
        std::option::IntoIter<EffectKind>,
        std::iter::Copied<std::iter::Flatten<std::option::IntoIter<&'a BTreeSet<EffectKind>>>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.direct
            .filter(|kind| {
                self.inherited
                    .is_none_or(|inherited| !inherited.contains(kind))
            })
            .into_iter()
            .chain(self.inherited.into_iter().flatten().copied())
    }
}

fn check_roster(observed: &mutations::Observations) -> Result<(), String> {
    let boundaries: BTreeSet<_> = observed
        .calls
        .iter()
        .map(|call| call.boundary_index)
        .collect();
    if boundaries.len() != observed.calls.len() {
        return Err("Density helper calls share one original boundary".into());
    }
    observed
        .calls
        .iter()
        .try_for_each(|call| check_boundary(observed, call))
}
