//! Borrowed original owners resolve retained choices without supplying rule evidence.
use super::{CompilationTarget, Locator, local, status};
use std::collections::BTreeMap;

pub(super) type Assessment = (status::State, String);

pub(super) trait Owner {
    fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str>;
}

#[derive(Clone, Copy)]
pub(super) enum Domain {
    Source,
    Mutation,
    Density,
}

impl Domain {
    fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Mutation => "Mutation",
            Self::Density => "Density",
        }
    }

    fn retained_label(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Mutation => "mutation",
            Self::Density => "density",
        }
    }

    fn unavailable(self, detail: &str) -> Assessment {
        (
            status::State::Unavailable,
            format!("{} {detail}", self.label()),
        )
    }
}

pub(super) struct Population<'a, R> {
    owners: BTreeMap<(&'a Locator, &'a CompilationTarget), Vec<&'a R>>,
    uncertain: bool,
    truncated: bool,
}

impl<'a, R: Owner> Population<'a, R> {
    pub(super) fn new(rows: &'a [R], unavailable_scope: bool, truncated: bool) -> Self {
        let mut owners = BTreeMap::<_, Vec<_>>::new();
        let mut uncertain = unavailable_scope;
        for row in rows {
            match row.identity() {
                Ok((locator, context)) => owners
                    .entry((locator, &context.compilation_target))
                    .or_default()
                    .push(row),
                Err(_) => uncertain = true,
            }
        }
        Self {
            owners,
            uncertain,
            truncated,
        }
    }

    pub(super) fn resolve(
        &self,
        locator: &Locator,
        context: &local::BuildContext,
        domain: Domain,
    ) -> Result<&'a R, Assessment> {
        if self.truncated {
            return Err(domain.unavailable("identity budget exhausted; uniqueness is unknown"));
        }
        let Some(owners) = self.owners.get(&(locator, &context.compilation_target)) else {
            return Err(self.missing(domain));
        };
        if owners.len() != 1 {
            return Err(domain.unavailable("owner has ambiguous observations in this target"));
        }
        let owner = owners[0];
        let Ok((_, observed)) = owner.identity() else {
            return Err(self.missing(domain));
        };

        if context != observed {
            return Err(if self.uncertain {
                self.missing(domain)
            } else {
                (
                    status::State::ContextChanged,
                    format!(
                        "{} owner was observed in a different build context",
                        domain.label()
                    ),
                )
            });
        }
        Ok(owner)
    }

    fn missing(&self, domain: Domain) -> Assessment {
        if self.uncertain {
            domain
                .unavailable("identity scope is incomplete; a retained owner cannot be classified")
        } else {
            (
                status::State::NotObserved,
                format!(
                    "Retained {} owner was not observed; deletion and omitted capture scope are not distinguished",
                    domain.retained_label()
                ),
            )
        }
    }
}
