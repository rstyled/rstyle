//! Function evidence and unit construction results are immutable and policy-local.
mod reference_origins;
pub(super) use reference_origins::Queries as ReferenceOrigins;

use super::*;
use crate::{
    inherent_calls::Definition,
    nominal_boundary::{
        census::Census,
        resolution::{Environment, Resolution, ResolutionView, SourceOptions as SourcePolicy},
    },
};
use std::{borrow::Cow, cell::OnceCell, rc::Rc, sync::Arc};

/// Each original source family performs its own complete check before ownership wrapping.
pub(super) trait OriginalSource: Clone {
    fn check(facts: &dyn EvidenceFacts) -> Result<Option<Self>, String>;
}

impl OriginalSource for GuardedReaderSource {
    fn check(facts: &dyn EvidenceFacts) -> Result<Option<Self>, String> {
        crate::data_documents::fallible_readers::check_source(facts)
    }
}

impl OriginalSource for Definition {
    fn check(facts: &dyn EvidenceFacts) -> Result<Option<Self>, String> {
        crate::inherent_calls::check_definition(facts)
    }
}

pub(super) fn owned_source<'a, T: OriginalSource + 'a>(
    facts: &(impl EvidenceFacts + ?Sized),
) -> Result<Option<Cow<'a, T>>, String> {
    T::check(facts.facts()).map(|proof| proof.map(Cow::Owned))
}

#[derive(Default)]
pub(super) struct Derived {
    // Source-only outcomes are stable even while helper certification progresses.
    // An error remains an error, never a Resolution or admission witness.
    nominal: Checked<Result<Resolution, String>>,
    data_frontend: Checked<()>,
    // Only successful dependency checks survive append-only helper installation.
    // Recapture starts fresh slots when source facts or helper links change.
    widening_authority: Checked<Box<WideningReaderAuthority>>,
    guarded_source: Checked<Box<GuardedReaderSource>>,
    receivers: [Checked<crate::data_receivers::Capture>; 2],
    inherent: Checked<Option<Definition>>,
    reference_origins: reference_origins::Queries,
}

pub(super) struct Checked<T>([OnceCell<T>; 2]);

impl<T> Default for Checked<T> {
    fn default() -> Self {
        Self([OnceCell::new(), OnceCell::new()])
    }
}

impl<T> Checked<T> {
    pub(super) fn get(&self, check: impl FnOnce() -> Result<T, String>) -> Result<&T, String> {
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
pub(super) struct Sources {
    contexts: RefCell<Vec<SourceContext>>,
    dependency_namespace_capture:
        OnceCell<Result<Option<Arc<crate::dependency_namespaces::capture::UnitCapture>>, String>>,
    generated_capture:
        OnceCell<Result<Option<Arc<crate::generated_sources::capture::UnitCapture>>, String>>,
}

fn unit_generated_capture(
    functions: &[HirFnFacts],
) -> Result<Option<Arc<crate::generated_sources::capture::UnitCapture>>, String> {
    crate::frontend::unit_records::single(
        functions,
        |facts| facts.generated_source_capture(),
        "Owning unit has duplicate or conflicting generated-source captures",
    )
}

struct SourceContext {
    census: Arc<Census>,
    constants: bool,
    expansions: bool,
    module_lints: bool,
    dependency_owner: bool,
    namespace_capability: bool,
    external_macros: bool,
    generated_namespaces: bool,
    cfgs: Option<Vec<String>>,
    checked: Checked<Result<Rc<Environment>, String>>,
}

impl SourceContext {
    fn checked(&self, input: Option<&dyn EvidenceFacts>) -> Result<Rc<Environment>, String> {
        self.checked.get(|| Ok(self.environment(input)))?.clone()
    }

    fn environment(&self, input: Option<&dyn EvidenceFacts>) -> Result<Rc<Environment>, String> {
        let environment = Environment::with_source_options(
            Arc::clone(&self.census),
            SourcePolicy {
                constants: self.constants,
                cfgs: self.cfgs.as_deref(),
                expansions: self.expansions,
                module_lints: self.module_lints,
                generated_namespaces: self.generated_namespaces,
            },
        )?;
        let environment = match input {
            Some(input) => environment.with_dependency_namespaces(input)?,
            None => environment,
        };
        Ok(Rc::new(environment))
    }
}

impl Sources {
    pub(super) fn dependency_namespace_capture(
        &self,
        functions: &[HirFnFacts],
    ) -> Result<Option<Arc<crate::dependency_namespaces::capture::UnitCapture>>, String> {
        crate::frontend::unit_records::retained(&self.dependency_namespace_capture, || {
            crate::dependency_namespaces::unit_capture(functions)
        })
    }

    /// One scan of the immutable owning table, independent of nominal resolution.
    pub(super) fn generated_capture(
        &self,
        functions: &[HirFnFacts],
    ) -> Result<Option<Arc<crate::generated_sources::capture::UnitCapture>>, String> {
        crate::frontend::unit_records::retained(&self.generated_capture, || {
            unit_generated_capture(functions)
        })
    }

    fn environment(&self, input: &dyn EvidenceFacts) -> Result<Rc<Environment>, String> {
        self.source_environment(input.facts(), Some(input))
    }

    /// Reuse the producer's capability-gated complete source proof without
    /// granting any consumer dependency, callable or nominal permissions.
    pub(super) fn generated_environment(
        &self,
        facts: &HirFnFacts,
    ) -> Result<Rc<Environment>, String> {
        self.source_environment(facts, None)
    }

    fn source_environment(
        &self,
        facts: &HirFnFacts,
        input: Option<&dyn EvidenceFacts>,
    ) -> Result<Rc<Environment>, String> {
        crate::nominal_boundary::census::policy::authority(facts)?;
        let census = facts.build.frontend_options.data::<Arc<Census>>()?;
        self.for_input(
            &census,
            SourcePolicy {
                generated_namespaces: crate::extraction_profile::facts_support(
                    facts,
                    crate::dependency_namespaces::GENERATED_CAPABILITY,
                ),
                constants: crate::nominal_boundary::resolution::constant_context(facts)?,
                cfgs: crate::nominal_boundary::resolution::selected_cfgs(facts)?,
                expansions: crate::extraction_profile::facts_support(
                    facts,
                    "rust.cfg-if-module-ownership",
                ),
                module_lints: crate::extraction_profile::facts_support(
                    facts,
                    crate::concrete_types::EXPLICIT_U8_CAPABILITY,
                ),
            },
            input,
        )
    }

    #[cfg(test)]
    fn for_census(
        &self,
        census: &Arc<Census>,
        policy: SourcePolicy<'_>,
    ) -> Result<Rc<Environment>, String> {
        self.for_input(census, policy, None)
    }

    fn for_input(
        &self,
        census: &Arc<Census>,
        policy: SourcePolicy<'_>,
        input: Option<&dyn EvidenceFacts>,
    ) -> Result<Rc<Environment>, String> {
        let SourcePolicy {
            constants,
            cfgs,
            expansions,
            module_lints,
            generated_namespaces,
        } = policy;
        let namespace_capability = input_supports(input, crate::dependency_namespaces::CAPABILITY);
        let external_macros =
            input_supports(input, "rust.checked-external-expression-macro-ownership");

        let mut contexts = self.contexts.borrow_mut();
        let index = if let Some(index) = contexts.iter().position(|context| {
            Arc::ptr_eq(&context.census, census)
                && context.constants == constants
                && context.expansions == expansions
                && context.module_lints == module_lints
                && context.generated_namespaces == generated_namespaces
                && context.dependency_owner == input.is_some()
                && context.namespace_capability == namespace_capability
                && context.external_macros == external_macros
                && context.cfgs.as_deref() == cfgs
        }) {
            index
        } else {
            contexts.push(SourceContext {
                census: Arc::clone(census),
                constants,
                expansions,
                module_lints,
                dependency_owner: input.is_some(),
                namespace_capability,
                external_macros,
                generated_namespaces,
                cfgs: cfgs.map(<[String]>::to_vec),
                checked: Checked::default(),
            });
            contexts.len() - 1
        };
        contexts[index].checked(input)
    }

    fn resolve_observed(&self, input: &dyn EvidenceFacts) -> Result<Resolution, String> {
        crate::diagnostics::Stage::new("source.nominal_shared", &input.facts().name, None)
            .enter()
            .finish(self.resolve(input))
    }

    // Both outcomes are retained by FunctionView::nominal. Keep this proof
    // independent of helper tables that change during Builder::capture.
    pub(super) fn resolve(&self, input: &dyn EvidenceFacts) -> Result<Resolution, String> {
        let facts = input.facts();
        let interface = facts
            .build
            .frontend_options
            .data::<crate::concrete_types::InterfaceObservations>()?;
        crate::nominal_boundary::resolution::verify_in(input, &interface, self.environment(input)?)
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

pub(super) fn receivers<'a>(
    slots: &'a [Checked<crate::data_receivers::Capture>; 2],
    facts: &dyn EvidenceFacts,
    original_source: bool,
) -> Result<Cow<'a, crate::data_receivers::Capture>, String> {
    slots[usize::from(original_source)]
        .get(|| crate::data_receivers::check(facts, original_source))
        .map(Cow::Borrowed)
}

impl FunctionView<'_> {
    pub(super) fn source_inputs(&self) -> Result<(), String> {
        if self.slice_length_instance().is_some()
            || self.reference_instance().is_some()
            || self.reference_forwarder().is_some()
            || self.product_instance().is_some()
        {
            // Minting the private token verified the complete retained original
            // source owner. Immutable instance queries cannot change that proof.
            return Ok(());
        }
        if self.scalar_instance().is_some() {
            // The private instance constructor checks the exact original owner
            // after the complete scalar/body/bound/assertion proof succeeds.
            return Ok(());
        }
        self.resolver
            .sources()
            .environment(self)?
            .check_input_owner(self.facts())
    }

    pub(super) fn selected_origins(
        &self,
        node: &HirNode,
        selectors: &[usize],
    ) -> Result<Vec<crate::data_origins::Origin>, String> {
        self.derived()
            .reference_origins
            .select(node, selectors, self)
    }

    pub(super) fn widening_authority(&self) -> Result<Cow<'_, WideningReaderAuthority>, String> {
        self.derived()
            .widening_authority
            .get(|| crate::widening_readers::check_authority(self).map(Box::new))
            .map(|proof| Cow::Borrowed(proof.as_ref()))
    }

    pub(super) fn guarded_source(&self) -> Result<Option<Cow<'_, GuardedReaderSource>>, String> {
        if !crate::data_documents::fallible_readers::candidate(self) {
            return Ok(None);
        }
        self.derived()
            .guarded_source
            .get(|| {
                crate::data_documents::fallible_readers::check_source(self)?
                    .map(Box::new)
                    .ok_or_else(|| "Missing candidate guarded reader source".into())
            })
            .map(|proof| Some(Cow::Borrowed(proof.as_ref())))
    }

    pub(super) fn data_boundary(&self) -> Result<(), String> {
        self.derived()
            .data_frontend
            .get(|| crate::data_frontend::check(self))
            .copied()
    }

    pub(super) fn receivers(
        &self,
        original_source: bool,
    ) -> Result<Cow<'_, crate::data_receivers::Capture>, String> {
        receivers(&self.derived().receivers, self, original_source)
    }

    pub(super) fn option_type(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        self.resolver
            .sources()
            .environment(self)?
            .verify_option_type(self.facts(), path, scalar)
    }

    pub(super) fn nominal(&self) -> Result<ResolutionView<'_>, String> {
        self.derived()
            .nominal
            .get(|| Ok(self.resolver.sources().resolve_observed(self)))?
            .as_ref()
            .map(ResolutionView::borrowed)
            .map_err(Clone::clone)
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

    fn policy(constants: bool) -> SourcePolicy<'static> {
        SourcePolicy {
            constants,
            cfgs: None,
            expansions: false,
            module_lints: false,
            generated_namespaces: false,
        }
    }

    #[test]
    fn expanded_source_policy_does_not_reuse_ordinary_source_context() {
        let census = crate::nominal_boundary::resolution::environment::tests::census();
        let sources = Sources::default();
        let ordinary = sources.for_census(&census, policy(false)).unwrap();
        let expanded = sources
            .for_census(
                &census,
                SourcePolicy {
                    expansions: true,
                    ..policy(false)
                },
            )
            .unwrap();
        assert!(!Rc::ptr_eq(&ordinary, &expanded));
        let linted = sources
            .for_census(
                &census,
                SourcePolicy {
                    module_lints: true,
                    ..policy(false)
                },
            )
            .unwrap();
        assert!(!Rc::ptr_eq(&ordinary, &linted));
    }

    #[test]
    fn generated_namespace_source_policy_is_isolated_and_reused() {
        let census = crate::nominal_boundary::resolution::environment::tests::census();
        for first in [false, true] {
            let sources = Sources::default();
            let selected = |enabled| SourcePolicy {
                generated_namespaces: enabled,
                ..policy(false)
            };
            let one = sources.for_census(&census, selected(first)).unwrap();
            let two = sources.for_census(&census, selected(!first)).unwrap();
            assert!(!Rc::ptr_eq(&one, &two));
            assert!(Rc::ptr_eq(
                &one,
                &sources.for_census(&census, selected(first)).unwrap()
            ));
        }
    }

    #[test]
    fn dependency_namespace_capability_is_part_of_the_source_cache_context() {
        let census = crate::nominal_boundary::resolution::environment::tests::census();
        let mut current: HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        current.build.frontend_options.clear();
        current.build.frontend_options.insert(
            crate::extraction_profile::SELECTION_KEY.into(),
            "hir-data-v3".into(),
        );
        let mut historical = current.clone();
        historical.build.frontend_options.insert(
            crate::extraction_profile::SELECTION_KEY.into(),
            "hir-data-v2".into(),
        );
        for (first, second) in [(&current, &historical), (&historical, &current)] {
            let sources = Sources::default();
            let one = sources
                .for_input(&census, policy(false), Some(first))
                .unwrap();
            let two = sources
                .for_input(&census, policy(false), Some(second))
                .unwrap();
            assert!(!Rc::ptr_eq(&one, &two));
            assert!(Rc::ptr_eq(
                &one,
                &sources
                    .for_input(&census, policy(false), Some(first))
                    .unwrap()
            ));
        }
    }

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
        let first = sources.for_census(&census, policy(false)).unwrap();
        assert!(Rc::ptr_eq(
            &first,
            &sources.for_census(&census, policy(false)).unwrap()
        ));
        crate::verification_policy::generated(|| {
            let generated = sources.for_census(&census, policy(false)).unwrap();
            assert!(!Rc::ptr_eq(&first, &generated));
            assert!(Rc::ptr_eq(
                &generated,
                &sources.for_census(&census, policy(false)).unwrap()
            ));
        });
        assert!(Rc::ptr_eq(
            &first,
            &sources.for_census(&census, policy(false)).unwrap()
        ));
        assert!(!Rc::ptr_eq(
            &first,
            &Sources::default()
                .for_census(&census, policy(false))
                .unwrap()
        ));
        let constants = sources.for_census(&census, policy(true)).unwrap();
        assert!(!Rc::ptr_eq(&first, &constants));
        assert!(Rc::ptr_eq(
            &constants,
            &sources.for_census(&census, policy(true)).unwrap()
        ));
        let mut changed = (*census).clone();
        changed.files[0].bytes = format!("{} ", changed.files[0].bytes).into();
        assert!(
            sources
                .for_census(&Arc::new(changed), policy(false))
                .is_err()
        );
    }

    #[test]
    fn immutable_unit_failures_are_retained_without_becoming_function_evidence() {
        let mut census = crate::nominal_boundary::resolution::environment::tests::census()
            .as_ref()
            .clone();
        census.files[0].digest = "sha256:invalid".into();
        let census = Arc::new(census);
        let sources = Sources::default();
        assert!(sources.for_census(&census, policy(false)).is_err());
        assert!(sources.for_census(&census, policy(false)).is_err());
        let contexts = sources.contexts.borrow();
        assert_eq!(contexts.len(), 1);
        assert!(contexts[0].checked.0[0].get().unwrap().is_err());
        assert!(contexts[0].checked.0[1].get().is_none());
        drop(contexts);
        crate::verification_policy::generated(|| {
            assert!(sources.for_census(&census, policy(false)).is_err());
        });
        assert!(
            sources.contexts.borrow()[0].checked.0[1]
                .get()
                .unwrap()
                .is_err()
        );
    }
}

#[cfg(test)]
mod nominal_tests;

fn input_supports(input: Option<&dyn EvidenceFacts>, capability: &str) -> bool {
    input.is_some_and(|input| crate::extraction_profile::facts_support(input.facts(), capability))
}
