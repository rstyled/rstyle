//! Bottom-up composition of owner summaries over resolved workspace calls.
//!
//! Each call contributes `count * callee[inputs := argument bounds]`. Every `Bound`
//! is monotone in its inputs, so substituting a per-invocation maximum for each
//! argument bounds the sum over invocations without indexed sums; the result may be
//! loose for per-item varying sizes but never undercounts. Standard-library bodies
//! are outside the cost unit. An unresolved, unavailable, recursive or unknown
//! callee contributes an opaque factor per invocation, and an unmappable callee
//! size stays opaque, so the caller keeps a sound partial bound.
//!
//! Library steps are a separate measure: each keyed library call contributes
//! `count * steps[sizes := argument bounds]` from its data contract, and an operation
//! without one contributes an opaque `?library` factor. They never change whether the
//! loop and invocation counts are proven.
pub use super::contracts::Contracts;
use super::symbolic::{Bound, OpaqueKind};
use super::*;
use crate::wording::{Message, Wording};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub const COST_SCOPE: &str = "workspace-loop-body-entries-and-invocations-through-resolved-calls";

/// What every composition assumes besides when it leaves a standard-library body out, which
/// `Wording::standard_bodies` says in each language's terms.
pub const ASSUMPTIONS: &[&str] = &[
    "Argument sizes are replaced by a per-invocation maximum; bounds are monotone, so totals are upper bounds but may be loose.",
    "Modeled standard iterator scans count as the owner's own loop-body entries.",
    "Composed counts still do not bound operation costs or whole runtime.",
    "Library steps add per-invocation contract bounds for keyed library operations; each contract holds up to a constant factor and excludes the cost of every element operation it counts.",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpaqueFactor {
    pub kind: OpaqueKind,
    pub site: String,
    pub reason: String,
}

/// A captured owner and its local outcome.
pub struct Analyzed {
    pub program: Program,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Composed {
    /// Every contribution is bounded by caller inputs and constants.
    Proven(Composition),
    /// A sound bound in which unproven loops, callees or sizes are opaque factors.
    Partial(Composition),
    Unknown {
        reason: String,
    },
    BudgetExceeded {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Composition {
    pub cost_scope: String,
    /// Why each opaque factor in the bounds is unknown, by site.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque_factors: Vec<OpaqueFactor>,
    pub loop_body_entries: Bound,
    pub workspace_invocations: Bound,
    /// Element operations in contracted library calls, through resolved callees.
    pub library_steps: Bound,
    /// Why each opaque factor in `library_steps` is unknown, by site.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_factors: Vec<OpaqueFactor>,
    pub resolved_callees: Vec<String>,
    pub excluded_library_callees: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Clone)]
struct Summary {
    totals: Totals,
    evidence: Evidence,
}

/// The composed measures of an owner, or of one call site's contribution.
#[derive(Clone)]
struct Totals {
    loops: Bound,
    invocations: Bound,
    library: Bound,
}

/// Everything composition records besides the measures.
#[derive(Clone, Default)]
struct Evidence {
    callees: BTreeSet<String>,
    library: BTreeSet<String>,
    // Inputs introduced by composition, keyed by label: a formal-rooted length path.
    synthesized: Introduced,
    opaque: Opaques,
    // Opaque factors of library steps, kept apart so they never affect the verdict.
    library_opaque: Opaques,
}

// Reasons for opaque factors, by kind and site.
type Opaques = BTreeMap<(OpaqueKind, String), String>;

enum State {
    Active,
    Done(Box<Result<Summary, Failure>>),
}

#[derive(Clone)]
enum Failure {
    Unknown(String),
    Budget(String),
}

fn factors(opaque: Opaques) -> Vec<OpaqueFactor> {
    opaque
        .into_iter()
        .map(|((kind, site), reason)| OpaqueFactor { kind, site, reason })
        .collect()
}

fn assumptions(wording: Wording) -> Vec<String> {
    let standard_bodies = std::iter::once(wording.text(Message::StandardBodies));
    standard_bodies
        .chain(ASSUMPTIONS.iter().copied())
        .map(String::from)
        .collect()
}

fn composition(summary: Summary, wording: Wording) -> Composition {
    Composition {
        cost_scope: COST_SCOPE.into(),
        opaque_factors: factors(summary.evidence.opaque),
        loop_body_entries: simplified(summary.totals.loops),
        workspace_invocations: simplified(summary.totals.invocations),
        library_steps: simplified(summary.totals.library),
        library_factors: factors(summary.evidence.library_opaque),
        resolved_callees: summary.evidence.callees.into_iter().collect(),
        excluded_library_callees: summary.evidence.library.into_iter().collect(),
        assumptions: assumptions(wording),
    }
}

/// Memoized bottom-up composition; owners are looked up and analyzed on demand.
pub struct Composer<F> {
    lookup: F,
    limits: Limits,
    contracts: Contracts,
    memo: Memo,
}

#[derive(Default)]
struct Memo {
    owners: BTreeMap<String, Option<Rc<Analyzed>>>,
    states: BTreeMap<String, State>,
}

impl<F: FnMut(&str) -> Option<Analyzed>> Composer<F> {
    /// A composer using the library contracts compiled into the engine.
    pub fn new(lookup: F, limits: Limits) -> Self {
        Self::with_contracts(lookup, limits, Contracts::embedded().clone())
    }

    pub fn with_contracts(lookup: F, limits: Limits, contracts: Contracts) -> Self {
        let memo = Memo::default();
        Self {
            lookup,
            limits,
            contracts,
            memo,
        }
    }

    /// What `id` composes to, with its assumptions in `wording`'s terms.
    pub fn composed(&mut self, id: &str, wording: Wording) -> Composed {
        match self.summary(id) {
            Ok(summary) if summary.evidence.opaque.is_empty() => {
                Composed::Proven(composition(summary, wording))
            }
            Ok(summary) => Composed::Partial(composition(summary, wording)),
            Err(Failure::Unknown(reason)) => Composed::Unknown { reason },
            Err(Failure::Budget(reason)) => Composed::BudgetExceeded { reason },
        }
    }

    fn summary(&mut self, id: &str) -> Result<Summary, Failure> {
        match self.memo.states.get(id) {
            Some(State::Done(result)) => return (**result).clone(),
            Some(State::Active) => {
                return Err(Failure::Unknown(format!(
                    "Recursive call cycle through {id}"
                )));
            }
            None => {}
        }
        if self.memo.states.len() >= self.limits.nodes.saturating_mul(16) {
            return Err(Failure::Budget("Composition owner budget exceeded".into()));
        }

        self.memo.states.insert(id.into(), State::Active);
        let result = self.build(id);
        self.memo
            .states
            .insert(id.into(), State::Done(Box::new(result.clone())));
        result
    }

    fn owner(&mut self, id: &str) -> Option<Rc<Analyzed>> {
        if !self.memo.owners.contains_key(id) {
            let analyzed = (self.lookup)(id).map(Rc::new);
            self.memo.owners.insert(id.into(), analyzed);
        }
        self.memo.owners[id].clone()
    }

    fn build(&mut self, id: &str) -> Result<Summary, Failure> {
        let owner = self
            .owner(id)
            .ok_or_else(|| Failure::Unknown(format!("No captured summary for callee {id}")))?;
        let assessment = match &owner.outcome {
            Outcome::Proven(assessment) | Outcome::Partial(assessment) => assessment,
            Outcome::Unknown { .. } => {
                return Err(Failure::Unknown(format!(
                    "Local bounds of {id} are unknown"
                )));
            }
            Outcome::BudgetExceeded { reason } => return Err(Failure::Budget(reason.clone())),
        };

        let mut summary = Summary {
            totals: Totals {
                loops: assessment.loop_body_entries.clone(),
                ..Totals::zero()
            },
            evidence: Evidence {
                opaque: local_opaque(assessment),
                ..Evidence::default()
            },
        };
        for call in &assessment.calls {
            self.call(&owner.program, call, &mut summary)?;
        }
        Ok(summary)
    }

    fn call(
        &mut self,
        caller: &Program,
        call: &CallBound,
        summary: &mut Summary,
    ) -> Result<(), Failure> {
        let contribution = match &call.target {
            Callee::Resolved(target) => self.contribution(caller, call, target),
            Callee::Constructor => return Ok(()),
            Callee::Library(identity) => {
                return self.library(Site::new(caller, call), identity, summary);
            }
            Callee::Unresolved => Err(Failure::Unknown(format!(
                "Unresolved callee {}",
                call.callee
            ))),
        };
        let contribution = contribution.unwrap_or_else(|failure| opaque_call(call, failure));
        summary.absorb(contribution, self.limits)
    }

    // The callee summary at this site, with its inputs replaced by caller bounds.
    fn contribution(
        &mut self,
        caller: &Program,
        call: &CallBound,
        target: &str,
    ) -> Result<Contribution, Failure> {
        let callee = self.summary(target)?;
        let owner = self
            .owner(target)
            .expect("summarized owners were looked up");
        let mapping = Mapping {
            site: Site::new(caller, call),
            callee: &owner.program,
            synthesized: &callee.evidence.synthesized,
        };

        let loops = mapping.substitute(&callee.totals.loops)?;
        let invocations = mapping.substitute(&callee.totals.invocations)?;
        let opaque = mapping.site.opaque.take();
        let library = mapping.substitute(&callee.totals.library)?;
        let library_opaque = mapping.site.opaque.take();
        let site = Evidence {
            callees: BTreeSet::from([target.to_string()]),
            synthesized: mapping.site.introduced.into_inner(),
            opaque,
            library_opaque,
            ..Evidence::default()
        };
        Ok(Contribution {
            count: call.count.clone(),
            totals: Totals {
                loops,
                invocations,
                library,
            },
            evidence: callee.evidence.inherited().union(site),
        })
    }

    // A library call costs its contract's steps at this site's argument sizes; it is
    // not a workspace invocation.
    fn library(
        &self,
        site: Site<'_>,
        identity: &str,
        summary: &mut Summary,
    ) -> Result<(), Failure> {
        let call = site.call;
        let contract = call
            .operation
            .as_deref()
            .and_then(|key| self.contracts.get(key));
        let steps = match contract {
            Some(contract) => contract.steps.instantiate(&mut |index| site.size(index))?,
            None => site.uncontracted(),
        };

        summary.totals.library = Bound::add(vec![
            summary.totals.library.clone(),
            Bound::mul(call.count.clone(), steps),
        ]);
        summary.record(Evidence {
            library: BTreeSet::from([identity.to_string()]),
            synthesized: site.introduced.into_inner(),
            library_opaque: site.opaque.into_inner(),
            ..Evidence::default()
        });
        summary.within(self.limits)
    }
}

struct Contribution {
    count: Bound,
    totals: Totals,
    // The callee's caller-visible evidence plus this site's callee, labels and factors.
    evidence: Evidence,
}

impl Totals {
    fn zero() -> Self {
        Self {
            loops: Bound::Const(0),
            invocations: Bound::Const(0),
            library: Bound::Const(0),
        }
    }

    // `count` invocations, each adding `part` and its own entry as an invocation.
    fn invoked(&self, count: &Bound, part: Self) -> Self {
        let per = |total: &Bound, each: Bound| {
            Bound::add(vec![total.clone(), Bound::mul(count.clone(), each)])
        };
        let entered = Bound::add(vec![Bound::Const(1), part.invocations]);
        Self {
            loops: per(&self.loops, part.loops),
            invocations: per(&self.invocations, entered),
            library: per(&self.library, part.library),
        }
    }

    fn size(&self) -> usize {
        self.loops.size() + self.invocations.size() + self.library.size()
    }
}

impl Evidence {
    fn union(self, other: Self) -> Self {
        Self {
            callees: joined(self.callees, other.callees),
            library: joined(self.library, other.library),
            synthesized: joined(self.synthesized, other.synthesized),
            opaque: joined(self.opaque, other.opaque),
            library_opaque: joined(self.library_opaque, other.library_opaque),
        }
    }

    // A callee's own synthesized labels are callee-relative; its call sites introduce
    // caller-relative ones instead.
    fn inherited(self) -> Self {
        Self {
            synthesized: Introduced::new(),
            ..self
        }
    }
}

fn joined<C: Extend<T> + IntoIterator<Item = T>, T>(mut first: C, second: C) -> C {
    first.extend(second);
    first
}

impl Summary {
    fn absorb(&mut self, part: Contribution, limits: Limits) -> Result<(), Failure> {
        self.totals = self.totals.invoked(&part.count, part.totals);
        self.record(part.evidence);
        self.within(limits)
    }

    fn record(&mut self, evidence: Evidence) {
        self.evidence = std::mem::take(&mut self.evidence).union(evidence);
    }

    fn within(&self, limits: Limits) -> Result<(), Failure> {
        if self.totals.size() > limits.expression_nodes {
            return Err(Failure::Budget("Composed bound budget exceeded".into()));
        }
        Ok(())
    }
}

/// Caller-side sizes of one call site's arguments.
struct Site<'a> {
    caller: &'a Program,
    call: &'a CallBound,
    // Caller-relative lengths this site introduces for the caller's own callers.
    introduced: RefCell<Introduced>,
    // Callee sizes without a caller-side bound, kept as opaque factors.
    opaque: RefCell<Opaques>,
}

/// Maps callee inputs to caller-side upper bounds at one call site.
struct Mapping<'a> {
    site: Site<'a>,
    callee: &'a Program,
    synthesized: &'a BTreeMap<String, (u32, Vec<Field>)>,
}

type Introduced = BTreeMap<String, (u32, Vec<Field>)>;

impl Mapping<'_> {
    fn substitute(&self, bound: &Bound) -> Result<Bound, Failure> {
        Ok(match bound {
            Bound::Const(value) => Bound::Const(*value),
            Bound::Opaque(opaque) => Bound::Opaque(opaque.clone()),
            Bound::Input(name) => self
                .input(name)
                .or_else(|failure| self.site.qualified(name, failure))?,
            Bound::Add(items) => Bound::add(self.all(items)?),
            Bound::Mul(items) => self
                .all(items)?
                .into_iter()
                .fold(Bound::Const(1), Bound::mul),
            Bound::Max(items) => Bound::Max(self.all(items)?),
            Bound::Log(value) => Bound::log(self.substitute(value)?),
        })
    }

    fn all(&self, items: &[Bound]) -> Result<Vec<Bound>, Failure> {
        items.iter().map(|item| self.substitute(item)).collect()
    }

    fn input(&self, name: &str) -> Result<Bound, Failure> {
        if let Some(position) = self.callee.formals.iter().position(
            |formal| matches!(&formal.measure, Measure::Scalar { input } if input == name),
        ) {
            return match self.site.call.arguments.get(position) {
                Some(ArgumentBound::Scalar(bound)) => Ok(bound.clone()),
                _ => Err(self
                    .site
                    .unmapped(name, "scalar argument has no upper bound")),
            };
        }

        let (formal, fields) = match self
            .callee
            .extents
            .iter()
            .find(|extent| extent.input == name)
        {
            Some(Extent {
                root: Root::Formal(formal),
                fields,
                ..
            }) => (*formal, fields.clone()),
            Some(_) => return Err(self.site.unmapped(name, "size is rooted at a callee local")),
            None => self
                .synthesized
                .get(name)
                .cloned()
                .ok_or_else(|| self.site.unmapped(name, "input has no formal provenance"))?,
        };
        self.site.length(name, (formal, &fields))
    }
}

impl<'a> Site<'a> {
    fn new(caller: &'a Program, call: &'a CallBound) -> Self {
        Self {
            caller,
            call,
            introduced: RefCell::new(BTreeMap::new()),
            opaque: RefCell::new(BTreeMap::new()),
        }
    }

    // Size of argument `index`: a scalar's bound, or a stable place's length.
    fn size(&self, index: u32) -> Result<Bound, Failure> {
        let name = format!("size({index})");
        match self.call.arguments.get(index as usize) {
            Some(ArgumentBound::Scalar(bound)) => Ok(bound.clone()),
            _ => self
                .length(&name, (index, &[]))
                .or_else(|failure| self.qualified(&name, failure)),
        }
    }

    // A library operation without a contract costs unknown steps per invocation.
    fn uncontracted(&self) -> Bound {
        let reason = match &self.call.operation {
            Some(key) => format!("No library contract for {key}"),
            None => "Library call has no operation key".into(),
        };
        let site = self.call.callee.clone();
        self.opaque
            .borrow_mut()
            .insert((OpaqueKind::Library, site.clone()), reason);
        Bound::opaque(OpaqueKind::Library, site)
    }

    // `origin` is the callee formal and the field path below it.
    fn length(&self, name: &str, origin: (u32, &[Field])) -> Result<Bound, Failure> {
        let (formal, fields) = origin;
        let Some(ArgumentBound::Place {
            root,
            fields: prefix,
        }) = self.call.arguments.get(formal as usize)
        else {
            return Err(self.unmapped(name, "argument is not a shared place"));
        };
        let path: Vec<Field> = prefix.iter().chain(fields).cloned().collect();
        if let Some(extent) = self
            .caller
            .extents
            .iter()
            .find(|extent| extent.root == *root && extent.fields == path)
        {
            return Ok(Bound::Input(extent.input.clone()));
        }

        // A shared formal's places cannot change during the caller's execution.
        let Root::Formal(position) = root else {
            return Err(self.unmapped(name, "caller place is not a stable extent"));
        };
        let Some(Formal {
            name: base,
            measure: Measure::Shared,
        }) = self.caller.formals.get(*position as usize)
        else {
            return Err(self.unmapped(name, "caller formal is not a shared reference"));
        };
        let label = synthesized_label(base, &path);
        self.introduced
            .borrow_mut()
            .insert(label.clone(), (*position, path));
        Ok(Bound::Input(label))
    }

    // An unmappable callee size is an unknown per-invocation maximum.
    fn qualified(&self, name: &str, failure: Failure) -> Result<Bound, Failure> {
        let Failure::Unknown(reason) = failure else {
            return Err(failure);
        };
        let site = format!("{}::{name}", self.call.callee);
        self.opaque
            .borrow_mut()
            .insert((OpaqueKind::Input, site.clone()), reason);
        Ok(Bound::opaque(OpaqueKind::Input, site))
    }

    fn unmapped(&self, name: &str, why: &str) -> Failure {
        Failure::Unknown(format!(
            "Callee input {name} at call to {} is unmapped: {why}",
            self.call.callee
        ))
    }
}

fn synthesized_label(base: &str, path: &[Field]) -> String {
    let fields: String = path
        .iter()
        .map(|field| format!(".{}", field.name))
        .collect();
    format!("len({base}{fields})")
}

// Flattens nested sums and products and folds their constants; meaning is unchanged.
fn simplified(bound: Bound) -> Bound {
    match bound {
        Bound::Add(items) => {
            let (constant, mut terms) = fold(items, 0, u64::saturating_add, |item| match item {
                Bound::Add(inner) => Ok(inner),
                other => Err(other),
            });
            terms.push(Bound::Const(constant));
            Bound::add(terms)
        }
        Bound::Mul(items) => {
            let (constant, factors) = fold(items, 1, u64::saturating_mul, |item| match item {
                Bound::Mul(inner) => Ok(inner),
                other => Err(other),
            });
            std::iter::once(Bound::Const(constant))
                .chain(factors)
                .reduce(Bound::mul)
                .unwrap_or(Bound::Const(1))
        }
        Bound::Max(items) => Bound::max(items.into_iter().map(simplified).collect()),
        Bound::Log(value) => Bound::log(simplified(*value)),
        leaf => leaf,
    }
}

fn fold(
    items: Vec<Bound>,
    unit: u64,
    combine: fn(u64, u64) -> u64,
    nested: fn(Bound) -> Result<Vec<Bound>, Bound>,
) -> (u64, Vec<Bound>) {
    let mut constant = unit;
    let mut terms = Vec::new();

    let flattened = items
        .into_iter()
        .map(simplified)
        .flat_map(|item| nested(item).unwrap_or_else(|leaf| vec![leaf]));
    for item in flattened {
        match item {
            Bound::Const(value) => constant = combine(constant, value),
            other => terms.push(other),
        }
    }
    (constant, terms)
}

// A callee that cannot be summarized costs an unknown amount per invocation.
fn opaque_call(call: &CallBound, failure: Failure) -> Contribution {
    let reason = match failure {
        Failure::Unknown(reason) | Failure::Budget(reason) => reason,
    };
    let site = call.callee.clone();
    let factor = |kind| ((kind, site.clone()), reason.clone());
    Contribution {
        count: call.count.clone(),
        totals: Totals {
            loops: Bound::opaque(OpaqueKind::Call, site.clone()),
            // Its own invocations are unknown too, beyond the one counted for this call.
            invocations: Bound::opaque(OpaqueKind::Invocations, site.clone()),
            library: Bound::opaque(OpaqueKind::Library, site.clone()),
        },
        evidence: Evidence {
            opaque: BTreeMap::from([factor(OpaqueKind::Call), factor(OpaqueKind::Invocations)]),
            library_opaque: BTreeMap::from([factor(OpaqueKind::Library)]),
            ..Evidence::default()
        },
    }
}

// An owner's own unproven loops, recorded even when a zero parent bound erased them.
fn local_opaque(assessment: &Assessment) -> Opaques {
    assessment
        .loops
        .iter()
        .filter_map(|item| item.opaque.as_ref())
        .map(|opaque| {
            (
                (OpaqueKind::Loop, opaque.site.clone()),
                opaque.reason.clone(),
            )
        })
        .collect()
}
