//! Successful derived evidence is local to an immutable function and policy.
use super::*;
use crate::{
    inherent_calls::Definition,
    nominal_boundary::{
        census::Census,
        resolution::{Environment, Resolution, ResolutionView},
    },
};
use std::{borrow::Cow, cell::OnceCell, rc::Rc, sync::Arc};

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

#[derive(Default)]
pub(super) struct Sources(RefCell<Vec<SourceContext>>);

struct SourceContext {
    census: Arc<Census>,
    constants: bool,
    checked: Checked<Rc<Environment>>,
}

impl Sources {
    fn environment(&self, facts: &HirFnFacts) -> Result<Rc<Environment>, String> {
        let census = facts.build.frontend_options.data::<Arc<Census>>()?;
        self.for_census(
            &census,
            crate::nominal_boundary::resolution::constant_context(facts)?,
        )
    }

    fn for_census(&self, census: &Arc<Census>, constants: bool) -> Result<Rc<Environment>, String> {
        let mut contexts = self.0.borrow_mut();
        let index = if let Some(index) = contexts.iter().position(|context| {
            Arc::ptr_eq(&context.census, census) && context.constants == constants
        }) {
            index
        } else {
            contexts.push(SourceContext {
                census: Arc::clone(census),
                constants,
                checked: Checked::default(),
            });
            contexts.len() - 1
        };
        contexts[index]
            .checked
            .get(|| Environment::with_constants(Arc::clone(census), constants).map(Rc::new))
            .map(Rc::clone)
    }

    fn resolve(&self, facts: &HirFnFacts) -> Result<Resolution, String> {
        let interface = facts
            .build
            .frontend_options
            .data::<crate::concrete_types::InterfaceObservations>()?;
        crate::nominal_boundary::resolution::verify_in(facts, &interface, self.environment(facts)?)
    }
}

pub(super) fn normalize(functions: Vec<HirFnFacts>) -> Result<Vec<HirFnFacts>, String> {
    let mut sources = Vec::new();
    let mut ranges = crate::source_table::Ranges::default();
    functions
        .into_iter()
        .map(|mut facts| {
            facts.build.frontend_options.normalize_data();
            facts.build.frontend_options.intern_census(&mut sources)?;
            facts.build.frontend_options.share_origin_sources()?;
            facts
                .build
                .frontend_options
                .share_declaration_ranges(&mut ranges)?;
            facts.source.bytes = facts.build.frontend_options.function_source(
                &facts.source,
                facts.fn_span,
                &mut ranges,
            )?;
            Ok(facts)
        })
        .collect()
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
            .get(|| self.resolver.sources().resolve(self.facts()))?;
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
    #[test]
    fn source_environments_are_owner_and_policy_local_and_reused() {
        let census = crate::nominal_boundary::resolution::environment::tests::census();
        let sources = Sources::default();
        let first = sources.for_census(&census, false).unwrap();
        assert!(Rc::ptr_eq(
            &first,
            &sources.for_census(&census, false).unwrap()
        ));
        crate::verification_policy::generated(|| {
            let generated = sources.for_census(&census, false).unwrap();
            assert!(!Rc::ptr_eq(&first, &generated));
            assert!(Rc::ptr_eq(
                &generated,
                &sources.for_census(&census, false).unwrap()
            ));
        });
        assert!(Rc::ptr_eq(
            &first,
            &sources.for_census(&census, false).unwrap()
        ));
        assert!(!Rc::ptr_eq(
            &first,
            &Sources::default().for_census(&census, false).unwrap()
        ));
        let constants = sources.for_census(&census, true).unwrap();
        assert!(!Rc::ptr_eq(&first, &constants));
        assert!(Rc::ptr_eq(
            &constants,
            &sources.for_census(&census, true).unwrap()
        ));
        let mut changed = (*census).clone();
        changed.files[0].bytes = format!("{} ", changed.files[0].bytes).into();
        assert!(sources.for_census(&Arc::new(changed), false).is_err());
    }
}
