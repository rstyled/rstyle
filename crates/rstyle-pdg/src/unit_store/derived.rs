//! Successful derived evidence is local to an immutable function and policy.
use super::*;
use crate::{
    inherent_calls::Definition,
    nominal_boundary::resolution::{Resolution, ResolutionView},
};
use std::{borrow::Cow, cell::OnceCell};

#[derive(Default)]
pub(super) struct Derived {
    nominal: Checked<Resolution>,
    inherent: Checked<Option<Definition>>,
}

struct Checked<T>([OnceCell<T>; 2]);

impl<T> Default for Checked<T> {
    fn default() -> Self {
        Self([OnceCell::new(), OnceCell::new()])
    }
}

impl<T> Checked<T> {
    fn get(&self, check: impl FnOnce() -> Result<T, String>) -> Result<&T, String> {
        let slot = &self.0[usize::from(crate::verification_policy::is_generated())];
        if let Some(value) = slot.get() {
            return Ok(value);
        }

        let value = check()?;
        slot.set(value)
            .map_err(|_| "Reentrant derived evidence verification")?;
        slot.get()
            .ok_or_else(|| "Missing checked derived evidence".into())
    }
}

pub(super) fn contexts(count: usize) -> Box<[Derived]> {
    std::iter::repeat_with(Derived::default)
        .take(count)
        .collect()
}

impl FunctionView<'_> {
    pub(super) fn nominal(&self) -> Result<ResolutionView<'_>, String> {
        let result = self
            .derived()
            .nominal
            .get(|| crate::nominal_boundary::resolution::from_facts(self.facts()))?;
        Ok(ResolutionView::borrowed(result))
    }

    pub(super) fn inherent(&self) -> Result<Option<Cow<'_, Definition>>, String> {
        let result = self
            .derived()
            .inherent
            .get(|| crate::inherent_calls::check_definition(self))?;
        Ok(result.as_ref().map(Cow::Borrowed))
    }

    fn derived(&self) -> &Derived {
        &self.resolver.derived()[self.id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn successful_results_are_borrowed_and_policy_and_owner_local() {
        let checked = Checked::default();
        let visits = Cell::new(0);
        let build = || {
            visits.set(visits.get() + 1);
            Ok::<_, String>(vec![1, 2, 3])
        };
        let strict = checked.get(build).unwrap();
        assert!(std::ptr::eq(strict, checked.get(build).unwrap()));
        crate::verification_policy::generated(|| {
            let generated = checked.get(build).unwrap();
            assert!(!std::ptr::eq(strict, generated));
            assert!(std::ptr::eq(generated, checked.get(build).unwrap()));
        });
        assert_eq!(visits.get(), 2);
        let independent = Checked::default();
        assert!(!std::ptr::eq(strict, independent.get(build).unwrap()));
        assert_eq!(visits.get(), 3);
    }

    #[test]
    fn failure_and_unwind_do_not_publish_derived_evidence() {
        let checked: Checked<Vec<u8>> = Checked::default();
        for _ in 0..2 {
            assert_eq!(
                checked
                    .get(|| Err("failed source proof".into()))
                    .unwrap_err(),
                "failed source proof"
            );
        }
        assert!(checked.0.iter().all(|slot| slot.get().is_none()));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::verification_policy::generated(|| {
                let _ = checked.get(|| panic!("interrupted source proof"));
            });
        }));
        assert!(panic.is_err());
        assert!(!crate::verification_policy::is_generated());
        assert!(checked.0.iter().all(|slot| slot.get().is_none()));
        assert_eq!(checked.get(|| Ok(vec![7])).unwrap(), &[7]);
        assert!(checked.0[1].get().is_none());
    }
}
