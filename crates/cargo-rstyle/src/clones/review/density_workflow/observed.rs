//! Density adapters retain checked owner identity and completed numeric snapshots.
use super::super::{
    Locator, density_rules, local, local_adapter, owner_observations, owner_population, workflow,
};
use crate::clones::{FunctionKey, analysis::effect_reports, report};

type Identity = (Locator, local::BuildContext);

pub(in crate::clones::review) type Observations = owner_observations::Observations<Row>;

pub(in crate::clones::review) struct Row {
    pub key: FunctionKey,
    pub selected: bool,
    pub complete: bool,
    pub visible: bool,
    unique: bool,
    witness: Result<Witness, String>,
}

enum Witness {
    Identity(Box<IdentityWitness>),
    Finding(Box<density_rules::Snapshot>),
}

struct IdentityWitness {
    identity: Identity,
    policy: density_rules::Policy,
}

impl Row {
    fn new(
        source: effect_reports::Review,
        selected: bool,
        identity: Result<Identity, String>,
    ) -> Self {
        let witness = identity.map(|identity| {
            if source.complete
                && source.visible
                && let Some(measurement) = source.measurement
            {
                return Witness::Finding(Box::new(density_rules::Snapshot {
                    versions: local_adapter::versions("side_effect_density"),
                    locator: identity.0,
                    context: identity.1,
                    policy: source.policy,
                    measurement,
                }));
            }
            Witness::Identity(Box::new(IdentityWitness {
                identity,
                policy: source.policy,
            }))
        });
        Self {
            key: source.key,
            selected,
            complete: source.complete,
            visible: source.visible,
            unique: true,
            witness,
        }
    }

    pub fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        match self.witness.as_ref().map_err(String::as_str)? {
            Witness::Identity(witness) => Ok((&witness.identity.0, &witness.identity.1)),
            Witness::Finding(snapshot) => Ok((&snapshot.locator, &snapshot.context)),
        }
    }

    pub fn policy(&self) -> Result<&density_rules::Policy, &str> {
        match self.witness()? {
            Witness::Identity(witness) => Ok(&witness.policy),
            Witness::Finding(snapshot) => Ok(&snapshot.policy),
        }
    }

    pub fn snapshot(&self) -> Result<&density_rules::Snapshot, String> {
        match self.witness()? {
            Witness::Finding(snapshot) => Ok(snapshot),
            Witness::Identity(_) => Err("Owner has no completed visible density finding".into()),
        }
    }

    fn witness(&self) -> Result<&Witness, &str> {
        self.witness.as_ref().map_err(String::as_str)
    }

    pub fn checked(&self) -> Result<density_rules::LiveFinding<'_>, String> {
        if !self.complete || !self.visible || !self.unique {
            return Err("Density assessment or original owner uniqueness is unavailable".into());
        }
        density_rules::LiveFinding::checked(self.snapshot()?).map_err(|error| format!("{error:?}"))
    }
}

pub(in crate::clones::review) fn collect(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Observations {
    owner_observations::collect_selected(inputs.effects, inputs, ctx, Row::new)
}

impl owner_population::Owner for Row {
    fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        Row::identity(self)
    }
}

impl owner_observations::Observation for Row {
    fn key(&self) -> FunctionKey {
        self.key
    }

    fn set_unique(&mut self, unique: bool) {
        self.unique = unique;
    }
}

impl owner_observations::NumericObservation for Row {
    fn findings_complete(&self) -> bool {
        !self.visible || self.checked().is_ok()
    }
}
