//! Static policy events. Unknown boundaries never become a numeric lower bound.
use rstyle_core::{config::EffectPaths, facts::EffectKind};
use rstyle_pdg::{
    ByteSpan,
    consumer_facts::{Availability, effects, mutations},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub(in super::super) type Cache = BTreeMap<usize, BTreeSet<EffectKind>>;

#[derive(Serialize)]
pub(super) struct Inventory {
    pub coverage: mutations::Coverage,
    pub counts: BTreeMap<EffectKind, usize>,
    events: Vec<Event>,
    unresolved_boundaries: Vec<usize>,
}

#[derive(Serialize)]
struct Event {
    kind: EffectKind,
    span: ByteSpan,
    call_index: Option<usize>,
}

pub(super) fn inspect(
    session: &effects::Session<'_>,
    assessment: &effects::Assessment,
    paths: &EffectPaths,
    cache: &mut Cache,
) -> Result<Inventory, String> {
    let mut inventory = Inventory {
        coverage: mutations::Coverage::Incomplete,
        counts: BTreeMap::new(),
        events: Vec::new(),
        unresolved_boundaries: Vec::new(),
    };
    let Some(observed) = session.observations(assessment.function_index) else {
        return Ok(inventory);
    };
    inventory.direct_events(observed)?;
    let checked = checked_calls(observed.calls.len(), &assessment.calls)?;
    let resolved = inventory.collect_calls(
        observed,
        &checked,
        CallContext {
            session,
            paths,
            cache,
        },
    )?;
    let unresolved_boundaries: Vec<_> = (0..observed.direct.boundaries.len())
        .filter(|index| !resolved.contains(index))
        .collect();
    let complete = matches!(observed.direct.availability, Availability::Known(()))
        && matches!(observed.signature, Availability::Known(()))
        && observed.direct.execution == mutations::Execution::Completed
        && assessment.execution == mutations::Execution::Completed
        && unresolved_boundaries.is_empty();

    Ok(Inventory {
        coverage: if complete {
            mutations::Coverage::Complete
        } else {
            mutations::Coverage::Incomplete
        },
        unresolved_boundaries,
        ..inventory
    })
}

impl Inventory {
    fn collect_calls(
        &mut self,
        observed: &mutations::Observations,
        checked: &[Option<&effects::CallEvent>],
        context: CallContext<'_, '_>,
    ) -> Result<BTreeSet<usize>, String> {
        let CallContext {
            session,
            paths,
            cache,
        } = context;
        let occurrences = observed
            .calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                check_boundary(observed, call)?;
                let input = CallInput {
                    index,
                    call,
                    checked: checked[index],
                    paths,
                    session,
                };
                Ok(self
                    .call_events(input, cache)?
                    .then_some(call.boundary_index))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let resolved: BTreeSet<_> = occurrences.iter().flatten().copied().collect();

        if resolved.len() != occurrences.iter().flatten().count() {
            return Err("Density call observations share one original boundary".into());
        }
        Ok(resolved)
    }

    fn push(&mut self, kind: EffectKind, span: ByteSpan, call_index: Option<usize>) {
        *self.counts.entry(kind).or_default() += 1;
        self.events.push(Event {
            kind,
            span,
            call_index,
        });
    }

    fn direct_events(&mut self, observed: &mutations::Observations) -> Result<(), String> {
        let mut roots = BTreeMap::new();
        for site in observed
            .direct
            .sites
            .iter()
            .filter(|site| site.caller_visible)
        {
            let root = &site.root;
            let key = (root.native.krate, root.native.owner, root.native.local);
            if let Some(previous) = roots.insert(key, root) {
                if previous != root {
                    return Err("Density mutation roots contradict one native binding".into());
                }
            } else {
                self.push(EffectKind::Mutation, site.span, None);
            }
        }
        Ok(())
    }

    fn call_events(&mut self, input: CallInput<'_, '_>, cache: &mut Cache) -> Result<bool, String> {
        let CallInput {
            index,
            call,
            checked,
            paths,
            session,
        } = input;
        if let Some(checked) = checked {
            self.checked_events(
                CallInput {
                    index,
                    call,
                    checked: Some(checked),
                    paths,
                    session,
                },
                checked,
                cache,
            )?;
            return Ok(true);
        }

        if let Availability::Known(callee) = &call.resolved_callee
            && !callee.local
            && callee.safe
            && matches!(callee.result_type.as_str(), "u32" | "unit" | "!")
            && call.arguments.iter().all(|arg| {
                matches!(
                    arg.value,
                    Availability::Known(mutations::ArgumentValue::OwnedU32)
                )
            })
            && let Some(kind) = paths.classify(&callee.path)
        {
            self.push(kind, call.span, Some(index));
            return Ok(true);
        }
        self.push(EffectKind::Unknown, call.span, Some(index));
        Ok(false)
    }

    fn checked_events(
        &mut self,
        input: CallInput<'_, '_>,
        checked: &effects::CallEvent,
        cache: &mut Cache,
    ) -> Result<(), String> {
        let CallInput {
            index,
            call,
            paths,
            session,
            ..
        } = input;
        if checked.span != call.span || checked.native != call.native {
            return Err("Density summary call differs from its original observation".into());
        }
        if !checked.mappings.is_empty() {
            self.push(EffectKind::Mutation, call.span, Some(index));
        }
        if let std::collections::btree_map::Entry::Vacant(entry) =
            cache.entry(checked.summary_index)
        {
            entry.insert(summary_kinds(session, checked.summary_index, paths)?);
        }
        let inherited = &cache[&checked.summary_index];
        if let Some(kind) = call
            .target_path
            .as_deref()
            .and_then(|path| paths.classify(path))
            && !inherited.contains(&kind)
        {
            self.push(kind, call.span, Some(index));
        }
        for kind in inherited {
            self.push(*kind, call.span, Some(index));
        }

        Ok(())
    }
}

fn check_boundary(
    observed: &mutations::Observations,
    call: &mutations::Call,
) -> Result<(), String> {
    if observed
        .direct
        .boundaries
        .get(call.boundary_index)
        .is_none_or(|boundary| {
            boundary.span != call.span
                || boundary.availability != mutations::BoundaryAvailability::Unknown
        })
    {
        return Err("Density call is not bound to its original unknown boundary".into());
    }
    Ok(())
}

struct CallContext<'a, 'u> {
    session: &'a effects::Session<'u>,
    paths: &'a EffectPaths,
    cache: &'a mut Cache,
}

struct CallInput<'a, 'u> {
    index: usize,
    call: &'a mutations::Call,
    checked: Option<&'a effects::CallEvent>,
    paths: &'a EffectPaths,
    session: &'a effects::Session<'u>,
}

fn checked_calls(
    count: usize,
    calls: &[effects::CallEvent],
) -> Result<Vec<Option<&effects::CallEvent>>, String> {
    let mut indexed = vec![None; count];
    for call in calls {
        let slot = indexed
            .get_mut(call.call_index)
            .ok_or("Density summary call index is outside the original roster")?;
        if slot.replace(call).is_some() {
            return Err("Density summary repeats one original call occurrence".into());
        }
    }
    Ok(indexed)
}

fn summary_kinds(
    session: &effects::Session<'_>,
    index: usize,
    paths: &EffectPaths,
) -> Result<BTreeSet<EffectKind>, String> {
    let summary = session
        .summaries()
        .get(index)
        .ok_or("Density call lacks its checked scalar summary")?;
    let owner = session
        .observations(summary.function_index)
        .ok_or("Density summary lacks original call observations")?;
    Ok(owner
        .calls
        .iter()
        .filter_map(|inner| {
            inner
                .target_path
                .as_deref()
                .and_then(|path| paths.classify(path))
        })
        .collect())
}
