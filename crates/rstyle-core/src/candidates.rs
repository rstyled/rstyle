//! Layer 2 join: hole columns of function clusters (and enum dispatch inside one function)
//! are matched against layer-1 signature groups to propose traits, generic fns or parameters.

mod effects;
pub use effects::annotate_completed_effects;
use effects::effect_mismatch;

use crate::align::HoleKind;
use crate::cluster::{Cluster, Column, Params, cluster};
use crate::dispatch::{Dispatch, find};
use crate::facts::{EdgeKind, FnFacts, NodeKind, Pdg, Receiver};
use crate::shape::{normalize, short};
use crate::sigmine::SigGroup;
use crate::{ledger, locator, wl};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Version of the advisory proposal policy, independent of graph/metric versions.
pub const POLICY_VERSION: u32 = 2;
/// Inspectable policy rejection, independent of user review/ignore entries.
pub const FORWARDING_REASON: &str =
    "call-only forwarding through distinct single-use callee substitutions";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    TraitMethod,
    CapabilitySet,
    EnumDispatch,
    GenericFn,
    FnParam,
    Parameterize,
}

impl CandidateKind {
    /// The snake_case name used in reports and the ignore ledger.
    pub fn name(self) -> &'static str {
        match self {
            CandidateKind::TraitMethod => "trait_method",
            CandidateKind::CapabilitySet => "capability_set",
            CandidateKind::EnumDispatch => "enum_dispatch",
            CandidateKind::GenericFn => "generic_fn",
            CandidateKind::FnParam => "fn_param",
            CandidateKind::Parameterize => "parameterize",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Site {
    pub path: String,
    pub line: usize,
    pub id: String,
    pub name: String,
    /// Stable name for the ignore ledger (see `locator`); unlike `id` it survives impl-block
    /// renumbering.
    pub locator: String,
    /// Hex `wl::shape_hash` of the body's dependence graph; absent without a graph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
}

/// The score, split so each signal stays visible (support and lift are not summed blindly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Breakdown {
    /// Functions in the cluster, or arms of the match.
    pub support: usize,
    /// Specificity of the joined signature group (rare signatures are strong evidence).
    pub lift_milli: u32,
    /// Hole columns; many holes mean a weak template.
    pub holes: usize,
    pub coverage_milli: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// Stable fingerprint (see `ledger::candidate_id`); what `cargo rstyle ignore` takes.
    pub id: String,
    pub kind: CandidateKind,
    pub score_milli: u32,
    pub breakdown: Breakdown,
    pub observation: String,
    pub inference: String,
    pub possible_refactor: String,
    /// Every function that shows the pattern.
    pub sites: Vec<Site>,
    /// The functions that fill the method holes.
    pub definitions: Vec<Site>,
    pub counter_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suppressed {
    pub reason: String,
    pub sites: Vec<Site>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Mined {
    pub candidates: Vec<Candidate>,
    pub suppressed: Vec<Suppressed>,
}

/// Lookup tables over the corpus.
struct Index<'a> {
    by_id: BTreeMap<&'a str, &'a FnFacts>,
    groups: BTreeMap<&'a str, &'a SigGroup>,
    /// ADT ids that have methods in this workspace (the only types worth abstracting over).
    workspace_adts: BTreeSet<&'a str>,
    /// Owner type of every called fn seen in a PDG (impl blocks have no readable id segment).
    callee_owners: BTreeMap<&'a str, String>,
    /// See `cluster::Params::single_call_guard`.
    single_call_guard: bool,
    /// See `cluster::Params::max_holes`.
    max_holes: usize,
}

impl<'a> Index<'a> {
    fn new(facts: &'a [FnFacts], groups: &'a [SigGroup], params: &Params) -> Index<'a> {
        Index {
            single_call_guard: params.single_call_guard,
            max_holes: params.max_holes,
            by_id: facts.iter().map(|f| (f.id.as_str(), f)).collect(),
            groups: groups.iter().map(|g| (g.key.as_str(), g)).collect(),
            callee_owners: callee_owners(facts),
            workspace_adts: facts
                .iter()
                .filter_map(|f| f.sig.as_ref()?.self_ty.as_deref())
                .collect(),
        }
    }

    fn display(&self, value: &str) -> String {
        match self.by_id.get(value) {
            Some(f) => f.name.clone(),
            None => value.rsplit("::").next().unwrap_or(value).to_string(),
        }
    }

    /// Callee values: external ones keep their owner (`TcpStream::connect`, not `connect`).
    fn display_callee(&self, value: &str) -> String {
        match (self.by_id.get(value), self.callee_owners.get(value)) {
            (Some(f), _) => f.name.clone(),
            (None, Some(owner)) => format!("{owner}::{}", short(value)),
            (None, None) => last_two(value),
        }
    }
}

fn callee_owners(facts: &[FnFacts]) -> BTreeMap<&str, String> {
    let nodes = facts
        .iter()
        .filter_map(|f| f.pdg.as_ref())
        .flat_map(|p| &p.nodes);
    nodes
        .filter_map(|n| {
            let owner = n.callee_sig.as_ref()?.owner()?;
            Some((n.callee_id.as_deref()?, short_paths(&owner)))
        })
        .collect()
}

/// Last two `::` segments (`a::TcpStream::connect` -> `TcpStream::connect`), skipping
/// anonymous impl segments like `{impl#0}`.
fn last_two(id: &str) -> String {
    let parts: Vec<&str> = id.split("::").filter(|p| !p.starts_with('{')).collect();
    parts[parts.len().saturating_sub(2)..].join("::")
}

fn site(f: &FnFacts) -> Site {
    Site {
        path: f.path.clone(),
        line: f.line,
        id: f.id.clone(),
        name: f.name.clone(),
        locator: locator::base(f),
        shape: f
            .pdg
            .as_ref()
            .map(|p| format!("{:016x}", wl::shape_hash(p))),
    }
}

/// A method column that is already a trait method on every owner type.
#[derive(Debug, Clone)]
struct Existing {
    reason: String,
    trait_method: String,
    owners: BTreeSet<String>,
}

/// What a set of hole values (callee ids) amounts to.
enum Verdict<'a> {
    Trait {
        group: &'a SigGroup,
        defs: Vec<&'a FnFacts>,
    },
    Suppressed(Existing),
    Skip,
}

fn distinct(values: &[String]) -> Vec<&str> {
    let set: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    set.into_iter().collect()
}

fn same_trait_method(defs: &[&FnFacts]) -> Option<String> {
    let first = defs.first()?.implements.clone()?;
    defs.iter()
        .all(|f| f.implements.as_deref() == Some(first.as_str()))
        .then_some(first)
}

fn judge<'a>(ix: &Index<'a>, values: &[String]) -> Verdict<'a> {
    let wanted = distinct(values);
    let defs: Vec<&FnFacts> = wanted
        .iter()
        .filter_map(|v| ix.by_id.get(v).copied())
        .collect();
    let types: BTreeSet<String> = defs
        .iter()
        .filter_map(|f| f.sig.as_ref()?.owner())
        .collect();

    // Constructors are not a capability callers use polymorphically.
    let no_receiver = defs
        .iter()
        .all(|f| f.sig.as_ref().is_some_and(|s| s.receiver == Receiver::None));
    if no_receiver {
        return Verdict::Skip;
    }
    if defs.len() != wanted.len() || types.len() < 2 {
        return Verdict::Skip;
    }
    if let Some(trait_method) = same_trait_method(&defs) {
        let reason = format!(
            "all {} impls are of trait method {trait_method}",
            defs.len()
        );
        return Verdict::Suppressed(Existing {
            reason,
            trait_method,
            owners: types,
        });
    }
    let keys: BTreeSet<String> = defs
        .iter()
        .filter_map(|f| f.sig.as_ref().map(|s| normalize(s).key))
        .collect();
    let group = keys
        .iter()
        .next()
        .filter(|_| keys.len() == 1)
        .and_then(|k| ix.groups.get(k.as_str()));
    match group {
        Some(group) => Verdict::Trait { group, defs },
        None => Verdict::Skip,
    }
}

/// `1000 * ln(support)` plus lift, scaled by coverage, damped by many holes and effect mismatch.
pub fn score_milli(b: &Breakdown, mismatch: bool) -> u32 {
    let support = (b.support.max(1) as f64).ln() * 1000.0;
    let base = (support + f64::from(b.lift_milli)) * f64::from(b.coverage_milli) / 1000.0;
    let damped = if b.holes > 3 {
        base * 3.0 / b.holes as f64
    } else {
        base
    };
    (if mismatch { damped / 2.0 } else { damped }).round() as u32
}

fn sketch(key: &str, name: &str) -> String {
    let body = key.strip_prefix("fn(").unwrap_or(key);
    let body = body
        .strip_prefix("&mut Self")
        .map(|rest| format!("&mut self{rest}"))
        .or_else(|| {
            body.strip_prefix("&Self")
                .map(|rest| format!("&self{rest}"))
        })
        .or_else(|| body.strip_prefix("Self").map(|rest| format!("self{rest}")))
        .unwrap_or_else(|| body.to_string());
    format!("fn {name}({body};")
}

fn shared_name(defs: &[&FnFacts]) -> String {
    let short: BTreeSet<&str> = defs
        .iter()
        .map(|f| f.name.rsplit("::").next().unwrap_or(&f.name))
        .collect();
    match short.len() {
        1 => short.into_iter().next().unwrap_or("method").to_string(),
        _ => "method".to_string(),
    }
}

/// Finished evidence for one candidate, shared by cluster and dispatch paths.
struct Evidence<'a> {
    kind: CandidateKind,
    breakdown: Breakdown,
    sites: Vec<&'a FnFacts>,
    verdicts: Vec<(&'a SigGroup, Vec<&'a FnFacts>)>,
    summary: String,
    /// Trait method id of a suppressed column in the same cluster: extend that trait instead.
    existing_trait: Option<String>,
}

/// Drops module paths inside a rendered owner: `a::W<b::X>` -> `W<X>`.
fn short_paths(owner: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();

    for c in owner.chars() {
        if c.is_alphanumeric() || c == '_' || c == ':' {
            word.push(c);
        } else {
            out.push_str(word.rsplit("::").next().unwrap_or(&word));
            word.clear();
            out.push(c);
        }
    }
    out.push_str(word.rsplit("::").next().unwrap_or(&word));
    out
}

/// Trait of a trait method id: `crate::public::Signature::is_valid` -> `Signature`.
fn trait_name(trait_method: &str) -> String {
    let parts: Vec<&str> = trait_method.split("::").collect();
    let trait_path = &parts[..parts.len().saturating_sub(1)];
    trait_path.last().unwrap_or(&trait_method).to_string()
}

fn trait_text(ev: &Evidence<'_>) -> (String, String) {
    let methods: Vec<String> = ev
        .verdicts
        .iter()
        .map(|(group, defs)| sketch(&group.key, &shared_name(defs)))
        .collect();
    let types: BTreeSet<String> = ev.verdicts[0]
        .1
        .iter()
        .filter_map(|f| f.sig.as_ref()?.owner())
        .collect();
    let impls = types
        .iter()
        .map(|t| short_paths(t))
        .collect::<Vec<_>>()
        .join(", ");
    let inference = format!(
        "{impls} are used interchangeably through methods of identical signature: an implicit interface."
    );
    let refactor = match &ev.existing_trait {
        Some(t) => format!(
            "add {} to existing trait `{}`, already implemented by {impls}; make the sites generic over it",
            methods.join(" "),
            trait_name(t)
        ),
        None => format!(
            "trait Shared {{ {} }} implemented by {impls}; make the sites generic over it",
            methods.join(" ")
        ),
    };

    (inference, refactor)
}

fn finish(ev: Evidence<'_>) -> Candidate {
    let all_defs: Vec<&FnFacts> = ev
        .verdicts
        .iter()
        .flat_map(|(_, defs)| defs.iter().copied())
        .collect();
    let mismatch = ev
        .verdicts
        .iter()
        .filter_map(|(_, defs)| effect_mismatch(defs))
        .collect::<Vec<_>>();
    let mut counter = mismatch.clone();

    if ev.breakdown.lift_milli < 500 {
        counter.push("signature is common in this codebase (low specificity)".into());
    }
    let (inference, possible_refactor) = trait_text(&ev);
    let definitions: BTreeSet<Site> = all_defs.iter().map(|f| site(f)).collect();
    Candidate {
        id: String::new(),
        kind: ev.kind,
        score_milli: score_milli(&ev.breakdown, !mismatch.is_empty()),
        breakdown: ev.breakdown,
        observation: ev.summary,
        inference,
        possible_refactor,
        sites: ev.sites.iter().map(|f| site(f)).collect(),
        definitions: definitions.into_iter().collect(),
        counter_evidence: counter,
    }
}

/// `T0 = Dog | Cat; M0 = Dog::bark | Cat::meow`
fn columns_text(ix: &Index<'_>, columns: &[Column]) -> String {
    if columns.is_empty() {
        "no recorded substitutions".into()
    } else {
        columns_text_of(ix, &columns.iter().collect::<Vec<_>>())
    }
}

fn columns_text_of(ix: &Index<'_>, columns: &[&Column]) -> String {
    let parts: Vec<String> = columns
        .iter()
        .map(|c| {
            let shown: Vec<String> = distinct_in_order(&c.values)
                .iter()
                .map(|v| {
                    if method_like(c.kind) {
                        ix.display_callee(v)
                    } else {
                        ix.display(v)
                    }
                })
                .collect();
            format!("{} = {}", c.var, shown.join(" | "))
        })
        .collect();
    parts.join("; ")
}

fn distinct_in_order(values: &[String]) -> Vec<&String> {
    let mut seen = BTreeSet::new();
    values.iter().filter(|v| seen.insert(*v)).collect()
}

fn method_like(kind: HoleKind) -> bool {
    matches!(kind, HoleKind::Method | HoleKind::FreeFn)
}

enum Outcome<'a> {
    Found(Evidence<'a>),
    /// A candidate that needs no trait evidence (type-only clusters).
    Ready(Box<Candidate>),
    Hidden(Suppressed),
    Nothing,
}

fn sites_of<'a>(fns: &[&'a FnFacts], cluster: &Cluster) -> Vec<&'a FnFacts> {
    let mut sites: Vec<&FnFacts> = cluster.members.iter().map(|m| fns[*m]).collect();
    sites.sort_by(|a, b| (&a.path, a.line, &a.id).cmp(&(&b.path, b.line, &b.id)));
    sites
}

type Verdicts<'a> = Vec<(&'a SigGroup, Vec<&'a FnFacts>)>;

/// Judgement of the method-like columns: trait evidence, and the suppressed columns.
fn judge_columns<'a>(ix: &Index<'a>, cols: &[&Column]) -> (Verdicts<'a>, Vec<Existing>) {
    let mut found = Vec::new();
    let mut hidden = Vec::new();
    for col in cols {
        match judge(ix, &col.values) {
            Verdict::Trait { group, defs } => found.push((group, defs)),
            Verdict::Suppressed(existing) => hidden.push(existing),
            Verdict::Skip => {}
        }
    }
    (found, hidden)
}

fn lift_of(verdicts: &[(&SigGroup, Vec<&FnFacts>)]) -> u32 {
    verdicts
        .iter()
        .map(|(g, _)| g.score_milli)
        .min()
        .unwrap_or(0)
}

/// Fewer calls than this is a delegation or accessor, not a body worth generalizing
/// (real parallel bodies on tarpc/mundane have 2 or more, the noise has 1).
const MIN_GENERIC_CALLS: usize = 2;

/// Whether a trait method id lives in the same crate as the first site.
fn in_crate_of(trait_method: &str, sites: &[&FnFacts]) -> bool {
    let krate = |id: &str| id.split("::").next().map(str::to_string);
    sites
        .first()
        .is_some_and(|f| krate(&f.id) == krate(trait_method))
}

/// The only call is fed by parameters alone: a delegation shim (`fn len(&self) -> usize { self.0.len() }`).
fn forwards_params(f: &FnFacts) -> bool {
    let Some(pdg) = f.pdg.as_ref() else {
        return true;
    };
    let is_param = |n: usize| pdg.nodes[n].kind == NodeKind::Param;
    let calls = pdg
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::Call);
    calls.into_iter().all(|(call, _)| {
        let inputs = pdg
            .edges
            .iter()
            .filter(|e| e.to == call && e.kind == EdgeKind::Data);
        inputs.into_iter().all(|e| is_param(e.from))
    })
}

/// Enough calls to be worth generalizing. With the guard, a single call counts too unless it is a
/// pure parameter forward.
fn has_substance(ix: &Index<'_>, f: &FnFacts) -> bool {
    match call_count(f) {
        n if n >= MIN_GENERIC_CALLS => true,
        1 => ix.single_call_guard && !forwards_params(f),
        _ => false,
    }
}

fn call_count(f: &FnFacts) -> usize {
    f.pdg.as_ref().map_or(0, |p| {
        p.nodes.iter().filter(|n| n.kind == NodeKind::Call).count()
    })
}

fn summary_of(ix: &Index<'_>, sites: &[&FnFacts], cluster: &Cluster) -> String {
    format!(
        "{} functions have aligned dependence-graph regions; recorded substitutions: {}",
        sites.len(),
        columns_text(ix, &cluster.columns)
    )
}

fn breakdown_of(sites: &[&FnFacts], cluster: &Cluster, lift_milli: u32) -> Breakdown {
    let covers = &cluster.coverage_milli;
    Breakdown {
        support: sites.len(),
        lift_milli,
        holes: cluster.columns.len(),
        coverage_milli: covers.iter().sum::<u32>() / covers.len().max(1) as u32,
    }
}

/// A cluster whose only holes are workspace types (and unresolved callees): one generic fn
/// over those types. Ranked below trait evidence (score halved).
fn generic_fn(ix: &Index<'_>, sites: &[&FnFacts], cluster: &Cluster) -> Option<Candidate> {
    // Sites that are all impls of one workspace trait method are already abstracted.
    // Impls of an external trait (`Future::poll`) are forced by it, so duplication stays real.
    if same_trait_method(sites).is_some_and(|t| in_crate_of(&t, sites)) {
        return None;
    }
    if sites
        .iter()
        .any(|f| !has_substance(ix, f) || is_boilerplate(f))
    {
        return None;
    }
    let (types, others): (Vec<&Column>, Vec<&Column>) = cluster
        .columns
        .iter()
        .partition(|c| c.kind == HoleKind::Type);
    let own = |c: &Column| {
        c.values
            .iter()
            .all(|v| ix.workspace_adts.contains(v.as_str()))
    };
    // Callee holes must be external (`TcpStream::connect` vs `UnixStream::connect`): a
    // workspace callee that differs per type is behaviour, not just a type parameter.
    let external = |c: &&Column| {
        method_like(c.kind) && c.values.iter().all(|v| !ix.by_id.contains_key(v.as_str()))
    };
    if !types.iter().any(|c| own(c)) || !others.iter().all(external) {
        return None;
    }
    // External type holes (`TcpStream | UnixStream`) are often the ones that matter: show all.
    let holes = columns_text_of(ix, &types);
    let breakdown = breakdown_of(sites, cluster, 0);

    Some(Candidate {
        id: String::new(),
        kind: CandidateKind::GenericFn,
        score_milli: score_milli(&breakdown, false) / 2,
        breakdown,
        observation: summary_of(ix, sites, cluster),
        inference: "aligned graph regions use several workspace types".into(),
        possible_refactor: format!(
            "consider a generic definition over {holes}; verify type and call contracts"
        ),
        sites: sites.iter().map(|f| site(f)).collect(),
        definitions: vec![],
        counter_evidence: effect_mismatch(sites).into_iter().collect(),
    })
}

/// Code whose repetition is not a refactoring target: tests repeat arrange/act on purpose,
/// `main` is per-binary boilerplate, `ffi`/`raw` modules and `bindgen_*` items are thin
/// generated or hand-written shims over C.
fn is_boilerplate(f: &FnFacts) -> bool {
    let segments: Vec<&str> = f.id.split("::").collect();
    let in_tests =
        segments.contains(&"tests") || f.path.starts_with("tests/") || f.path.contains("/tests/");
    let ffi = segments
        .iter()
        .any(|s| matches!(*s, "ffi" | "raw") || s.starts_with("bindgen"));
    in_tests || ffi || segments.last() == Some(&"main")
}

/// Whether a cluster is worth a `parameterize` suggestion: not boilerplate, not impls forced
/// by a trait, and few enough holes to be a helper's parameters.
fn parameterizable(sites: &[&FnFacts], cluster: &Cluster, max_holes: usize) -> bool {
    let forced = sites.iter().all(|f| f.implements.is_some());
    !forced && !sites.iter().any(|f| is_boilerplate(f)) && cluster.columns.len() <= max_holes
}

/// A cluster that is neither a trait nor a generic fn over workspace types: aligned regions
/// with substitutions at external callees, literals or foreign types. A helper may share
/// those regions while preserving unmatched work. Ranked below `generic_fn`.
fn parameterize(ix: &Index<'_>, sites: &[&FnFacts], cluster: &Cluster) -> Option<Candidate> {
    if same_trait_method(sites).is_some_and(|t| in_crate_of(&t, sites)) {
        return None;
    }
    if cluster.forwarding_only
        || !parameterizable(sites, cluster, ix.max_holes)
        || sites.iter().any(|f| !has_substance(ix, f))
    {
        return None;
    }
    let breakdown = breakdown_of(sites, cluster, 0);
    let holes = columns_text(ix, &cluster.columns);

    Some(Candidate {
        id: String::new(),
        kind: CandidateKind::Parameterize,
        score_milli: score_milli(&breakdown, false) / 3,
        breakdown,
        observation: summary_of(ix, sites, cluster),
        inference: "aligned graph regions may support a shared computation".into(),
        possible_refactor: format!(
            "consider a helper for the aligned regions with these parameters: {holes}"
        ),
        sites: sites.iter().map(|f| site(f)).collect(),
        definitions: vec![],
        counter_evidence: effect_mismatch(sites).into_iter().collect(),
    })
}

/// No method hole joined a signature group: a type-only generic fn, a hidden pattern, or nothing.
fn without_trait<'a>(
    ix: &Index<'a>,
    sites: &[&'a FnFacts],
    cluster: &Cluster,
    hidden: Option<String>,
) -> Outcome<'a> {
    match hidden {
        Some(reason) if reason.starts_with("all ") => Outcome::Hidden(Suppressed {
            reason,
            sites: sites.iter().map(|f| site(f)).collect(),
        }),
        Some(_) => Outcome::Nothing,
        None if cluster.forwarding_only
            && parameterizable(sites, cluster, ix.max_holes)
            && sites.iter().all(|f| has_substance(ix, f)) =>
        {
            Outcome::Hidden(Suppressed {
                reason: FORWARDING_REASON.into(),
                sites: sites.iter().map(|f| site(f)).collect(),
            })
        }
        None => generic_fn(ix, sites, cluster)
            .or_else(|| parameterize(ix, sites, cluster))
            .map_or(Outcome::Nothing, |c| Outcome::Ready(Box::new(c))),
    }
}

fn from_cluster<'a>(ix: &Index<'a>, fns: &[&'a FnFacts], cluster: &Cluster) -> Outcome<'a> {
    let sites = sites_of(fns, cluster);
    let cols: Vec<&Column> = cluster
        .columns
        .iter()
        .filter(|c| method_like(c.kind))
        .collect();
    let (verdicts, hidden) = judge_columns(ix, &cols);

    if verdicts.is_empty() {
        let reason = hidden.last().map(|e| e.reason.clone());
        return without_trait(ix, &sites, cluster, reason);
    }
    let kind = if verdicts.len() > 1 {
        CandidateKind::CapabilitySet
    } else {
        CandidateKind::TraitMethod
    };
    Outcome::Found(Evidence {
        kind,
        breakdown: breakdown_of(&sites, cluster, lift_of(&verdicts)),
        summary: summary_of(ix, &sites, cluster),
        sites,
        existing_trait: existing_trait_for(&verdicts, &hidden),
        verdicts,
    })
}

fn owners_of(defs: &[&FnFacts]) -> BTreeSet<String> {
    defs.iter()
        .filter_map(|f| f.sig.as_ref()?.owner())
        .collect()
}

/// The suppressed column whose trait is implemented by the same owner types as the proposed
/// method: that is the trait to extend (an unrelated trait in the cluster is not).
fn existing_trait_for(verdicts: &Verdicts<'_>, hidden: &[Existing]) -> Option<String> {
    let wanted = owners_of(&verdicts.first()?.1);
    hidden
        .iter()
        .find(|e| e.owners == wanted)
        .map(|e| e.trait_method.clone())
}

fn from_dispatch<'a>(ix: &Index<'a>, f: &'a FnFacts, d: &Dispatch) -> Outcome<'a> {
    let values: Vec<String> = d.arms.iter().map(|(_, id)| id.clone()).collect();
    let summary = format!(
        "`{}` matches on an enum and calls analogous methods per arm: M0 = {}",
        f.name,
        values
            .iter()
            .map(|v| ix.display_callee(v))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    match judge(ix, &values) {
        Verdict::Trait { group, defs } => Outcome::Found(Evidence {
            kind: CandidateKind::EnumDispatch,
            breakdown: Breakdown {
                support: d.arms.len(),
                lift_milli: group.score_milli,
                holes: 1,
                coverage_milli: 1000,
            },
            sites: vec![f],
            verdicts: vec![(group, defs)],
            summary,
            existing_trait: None,
        }),
        Verdict::Suppressed(Existing { reason, .. }) => Outcome::Hidden(Suppressed {
            reason,
            sites: vec![site(f)],
        }),
        Verdict::Skip => Outcome::Nothing,
    }
}

impl Site {
    /// This site with its corpus-wide locator (duplicates numbered), keyed by fn id.
    fn named(self, names: &BTreeMap<String, String>) -> Site {
        let locator = names.get(&self.id).cloned().unwrap_or(self.locator);

        Site { locator, ..self }
    }
}

impl Candidate {
    /// The completed template/member node coverage says nothing about unmatched work.
    fn with_alignment_caveat(mut self) -> Self {
        if self.breakdown.coverage_milli < 1000 {
            self.counter_evidence.push(
                "Some graph nodes are unmatched; recorded substitutions describe aligned regions, not complete functions.".into(),
            );
            self.possible_refactor
                .push_str(" Preserve behavior outside the aligned graph regions.");
        }
        self
    }
}

impl Mined {
    /// Final locators for every site, then every candidate's id (which hashes them).
    fn identified(self, names: &BTreeMap<String, String>) -> Mined {
        let rename =
            |sites: Vec<Site>| -> Vec<Site> { sites.into_iter().map(|s| s.named(names)).collect() };
        let candidates = self.candidates.into_iter().map(|c| {
            let (sites, definitions) = (rename(c.sites), rename(c.definitions));
            let id = ledger::candidate_id(c.kind, &sites, &definitions);

            Candidate {
                id,
                sites,
                definitions,
                ..c
            }
        });
        let suppressed = self.suppressed.into_iter().map(|s| Suppressed {
            sites: rename(s.sites),
            ..s
        });

        Mined {
            candidates: candidates.collect(),
            suppressed: suppressed.collect(),
        }
    }

    fn with(mut self, outcome: Outcome<'_>) -> Mined {
        match outcome {
            Outcome::Found(ev) => self.candidates.push(finish(ev).with_alignment_caveat()),
            Outcome::Ready(c) => self.candidates.push((*c).with_alignment_caveat()),
            Outcome::Hidden(s) => self.suppressed.push(s),
            Outcome::Nothing => {}
        }
        self
    }

    /// Best first; ties by site, so output never depends on input order.
    fn sorted(mut self) -> Mined {
        self.candidates
            .sort_by(|a, b| (b.score_milli, &a.sites).cmp(&(a.score_milli, &b.sites)));
        self.suppressed
            .sort_by(|a, b| (&a.sites, &a.reason).cmp(&(&b.sites, &b.reason)));
        self
    }
}

fn collect(outcomes: Vec<Outcome<'_>>) -> Mined {
    outcomes
        .into_iter()
        .fold(Mined::default(), Mined::with)
        .sorted()
}

/// Candidates and suppressed patterns for `facts` (already prepared, see `prep`).
pub fn mine(facts: &[FnFacts], groups: &[SigGroup], params: &Params) -> Mined {
    let ix = Index::new(facts, groups, params);
    let mut fns: Vec<&FnFacts> = facts.iter().filter(|f| f.pdg.is_some()).collect();

    fns.sort_by(|a, b| (&a.path, a.line, &a.id).cmp(&(&b.path, b.line, &b.id)));
    let pdgs: Vec<&Pdg> = fns.iter().filter_map(|f| f.pdg.as_ref()).collect();
    let mut outcomes: Vec<Outcome<'_>> = cluster(&pdgs, params)
        .iter()
        .map(|c| from_cluster(&ix, &fns, c))
        .collect();
    for (f, pdg) in fns.iter().zip(&pdgs) {
        outcomes.extend(find(pdg).iter().map(|d| from_dispatch(&ix, f, d)));
    }
    collect(outcomes).identified(&locator::assign(facts))
}

#[cfg(test)]
pub(crate) mod tests_support {
    use crate::facts::{FnFacts, Receiver, sample};
    use crate::pdgtest::*;
    use crate::shape::fixtures::{method, string};

    pub fn getter(ty: &str, name: &str) -> FnFacts {
        FnFacts {
            id: format!("k::{ty}::{name}"),
            name: format!("{ty}::{name}"),
            path: format!("src/{}.rs", ty.to_lowercase()),
            sig: Some(method(&format!("k::{ty}"), Receiver::Ref, vec![], string())),
            ..sample(name)
        }
    }

    pub fn caller(ty: &str, name: &str) -> FnFacts {
        FnFacts {
            pdg: Some(emit_loop(ty, name)),
            path: "src/emit.rs".into(),
            line: 1000 - ty.bytes().map(usize::from).sum::<usize>(),
            ..sample(&format!("emit_{}", ty.to_lowercase()))
        }
    }

    pub fn corpus() -> Vec<FnFacts> {
        vec![
            getter("Dog", "bark"),
            getter("Cat", "meow"),
            caller("Dog", "bark"),
            caller("Cat", "meow"),
        ]
    }
}

#[cfg(test)]
mod tests {
    mod alignment_evidence;
    mod completed_effect_evidence;
    mod effect_evidence;
    mod forwarded_pipelines;
    use super::tests_support::{caller, corpus, getter};
    use super::*;
    use crate::facts::{MutationEvent, PdgNode, TyShape, sample};
    use crate::pdgtest::*;
    use crate::shape::fixtures::method;
    use crate::sigmine::mine as mine_groups;

    fn run(facts: &[FnFacts]) -> Mined {
        mine(facts, &mine_groups(facts), &Params::default())
    }

    #[test]
    fn method_hole_in_one_sig_group_proposes_trait() {
        let mined = run(&corpus());
        assert_eq!(mined.candidates.len(), 1);
        let c = &mined.candidates[0];
        assert_eq!(c.kind, CandidateKind::TraitMethod);
        assert!(
            c.observation.contains("M0 = Dog::bark | Cat::meow"),
            "{}",
            c.observation
        );
        assert!(
            c.possible_refactor.contains("fn method(&self) -> String"),
            "{}",
            c.possible_refactor
        );
        assert_eq!(c.breakdown.support, 2);
        assert_eq!(c.breakdown.coverage_milli, 1000);
        assert_eq!(c.definitions.len(), 2);
    }

    fn evidence<'a>(
        group: &'a SigGroup,
        defs: &[&'a FnFacts],
        existing_trait: Option<&str>,
    ) -> Evidence<'a> {
        Evidence {
            kind: CandidateKind::TraitMethod,
            breakdown: Breakdown {
                support: 2,
                lift_milli: 0,
                holes: 1,
                coverage_milli: 1000,
            },
            sites: vec![],
            verdicts: vec![(group, defs.to_vec())],
            summary: String::new(),
            existing_trait: existing_trait.map(String::from),
        }
    }

    #[test]
    fn last_two_keeps_the_owner_of_external_callees() {
        assert_eq!(
            last_two("tokio::net::tcp::TcpStream::connect"),
            "TcpStream::connect"
        );
        assert_eq!(last_two("connect"), "connect");
    }

    #[test]
    fn last_two_skips_anonymous_impl_segments() {
        assert_eq!(last_two("net::{impl#0}::connect"), "net::connect");
    }

    #[test]
    fn callee_owner_from_signature_beats_the_id_path() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for (f, owner) in facts[..2].iter_mut().zip(["Tcp", "Unix"]) {
            let mut node = getter_call(owner, "connect");
            node.callee_id = Some(format!("ext::{{impl#0}}::connect{owner}"));
            f.pdg.as_mut().expect("pdg").nodes[4] = node;
        }
        let c = &run(&facts).candidates[0];
        assert!(
            c.observation
                .contains("Tcp::connectTcp | Unix::connectUnix"),
            "{}",
            c.observation
        );
    }

    #[test]
    fn external_callee_holes_keep_their_owner_in_the_observation() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for (f, owner) in facts[..2].iter_mut().zip(["Tcp", "Unix"]) {
            f.pdg.as_mut().expect("pdg").nodes[4] = getter_call(owner, "connect");
        }
        let c = &run(&facts).candidates[0];
        assert!(
            c.observation.contains("M0 = Tcp::connect | Unix::connect"),
            "{}",
            c.observation
        );
    }

    #[test]
    fn short_paths_keeps_type_arguments_readable() {
        assert_eq!(short_paths("a::b::W<c::X, d::Y>"), "W<X, Y>");
        assert_eq!(short_paths("k::Dog"), "Dog");
    }

    #[test]
    fn trait_text_proposes_new_trait_without_existing_one() {
        let facts = [getter("Dog", "bark"), getter("Cat", "meow")];
        let groups = mine_groups(&facts);
        let defs: Vec<&FnFacts> = facts.iter().collect();
        let (_, refactor) = trait_text(&evidence(&groups[0], &defs, None));
        assert!(refactor.starts_with("trait Shared {"), "{refactor}");
    }

    #[test]
    fn trait_text_extends_existing_trait_when_one_is_in_the_cluster() {
        let facts = [getter("Dog", "bark"), getter("Cat", "meow")];
        let groups = mine_groups(&facts);
        let defs: Vec<&FnFacts> = facts.iter().collect();
        let ev = evidence(&groups[0], &defs, Some("k::public::Signature::is_valid"));
        let (_, refactor) = trait_text(&ev);
        assert!(
            refactor.starts_with("add fn method(&self) -> String; to existing trait `Signature`"),
            "{refactor}"
        );
        assert!(!refactor.contains("trait Shared"), "{refactor}");
    }

    /// Callers that differ only in the element type: both call one shared helper.
    fn type_only(ty: &str) -> FnFacts {
        let mut f = caller(ty, "x");
        let pdg = f.pdg.as_mut().expect("pdg");
        pdg.nodes[4] = getter_call("Shared", "get");
        f
    }

    fn type_only_corpus(types: [&str; 2]) -> Vec<FnFacts> {
        let owners = types.map(|t| getter(t, "own"));
        let mut facts = vec![type_only(types[0]), type_only(types[1])];
        facts.extend(owners);
        facts
    }

    #[test]
    fn type_only_cluster_proposes_generic_fn() {
        let c = &run(&type_only_corpus(["Dog", "Cat"])).candidates;
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!(c[0].kind, CandidateKind::GenericFn);
        assert!(
            c[0].possible_refactor.contains("T0 = Dog | Cat"),
            "{}",
            c[0].possible_refactor
        );
        assert_eq!(c[0].breakdown.lift_milli, 0);
        let half = score_milli(&c[0].breakdown, false) / 2;
        assert_eq!(c[0].score_milli, half);
    }

    fn no_generic_fn(facts: &[FnFacts]) -> bool {
        let kinds = run(facts)
            .candidates
            .iter()
            .map(|c| c.kind)
            .collect::<Vec<_>>();
        !kinds.contains(&CandidateKind::GenericFn)
    }

    /// Callers that differ only in an external callee: no workspace type, no trait.
    fn external_only(owner: &str, callee: &str) -> FnFacts {
        let mut f = caller("Dog", "x");
        let pdg = f.pdg.as_mut().expect("pdg");
        let sig = method(
            owner,
            Receiver::Ref,
            vec![],
            crate::shape::fixtures::string(),
        );
        pdg.nodes[4] = call(callee, sig);
        f
    }

    #[test]
    fn external_callee_only_cluster_proposes_a_parameterized_helper() {
        let facts = [
            external_only("ext::A", "ext::A::go"),
            external_only("ext::B", "ext::B::go"),
        ];
        let c = &run(&facts).candidates;
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!(c[0].kind, CandidateKind::Parameterize);
        assert!(
            c[0].possible_refactor.contains("A::go | B::go"),
            "{}",
            c[0].possible_refactor
        );
        let third = score_milli(&c[0].breakdown, false) / 3;
        assert_eq!(c[0].score_milli, third);
    }

    #[test]
    fn parameterize_skips_test_code_trait_impls_and_wide_clusters() {
        let pair = || {
            [
                external_only("ext::A", "ext::A::go"),
                external_only("ext::B", "ext::B::go"),
            ]
        };
        let mut tests = pair();
        tests
            .iter_mut()
            .for_each(|f| f.id = format!("k::tests::{}", f.id));
        assert!(run(&tests).candidates.is_empty());
        let mut forced = pair();
        forced
            .iter_mut()
            .for_each(|f| f.implements = Some("ext::Sink::poll_ready".into()));
        assert!(run(&forced).candidates.is_empty());
        let cluster = Cluster {
            forwarding_only: false,
            members: vec![0, 1],
            coverage_milli: vec![1000, 1000],
            columns: (0..=Params::default().max_holes)
                .map(|i| Column {
                    kind: HoleKind::Literal,
                    var: format!("L{i}"),
                    values: vec![],
                })
                .collect(),
        };
        let facts = pair();
        let sites: Vec<&FnFacts> = facts.iter().collect();
        let cap = Params::default().max_holes;
        assert!(!parameterizable(&sites, &cluster, cap));
        assert!(
            parameterizable(&sites, &cluster, cap + 1),
            "a raised cap admits the cluster"
        );
        for id in [
            "k::main",
            "k::raw::wrap",
            "k::ffi::wrap",
            "k::bindgen_test_layout_x",
        ] {
            let mut f = external_only("ext::A", "ext::A::go");
            f.id = id.into();
            assert!(is_boilerplate(&f), "{id}");
        }
    }

    #[test]
    fn generic_fn_skips_test_code() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for f in &mut facts[..2] {
            f.id = format!("k::tests::{}", f.id);
        }
        assert!(run(&facts).candidates.is_empty());
    }

    #[test]
    fn type_only_cluster_of_one_trait_method_is_already_abstracted() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for f in &mut facts[..2] {
            f.implements = Some("krate::Channel::config".into());
        }
        assert!(run(&facts).candidates.is_empty());
    }

    #[test]
    fn type_only_cluster_of_an_external_trait_method_is_still_emitted() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for f in &mut facts[..2] {
            f.implements = Some("futures_core::Future::poll".into());
        }
        assert_eq!(run(&facts).candidates.len(), 1);
    }

    #[test]
    fn generic_fn_text_shows_external_type_holes_too() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for (f, ext) in facts[..2].iter_mut().zip(["ext::Tcp", "ext::Unix"]) {
            let pdg = f.pdg.as_mut().expect("pdg");
            let mut stream = node(NodeKind::Param);
            stream.ty = Some(crate::shape::fixtures::adt(ext));
            pdg.nodes.push(stream);
            let last = pdg.nodes.len() - 1;
            pdg.edges.push(data(last, 3, 2));
        }
        let c = &run(&facts).candidates[0];
        assert!(
            c.possible_refactor.contains("T0 = Dog | Cat"),
            "{}",
            c.possible_refactor
        );
        assert!(
            c.possible_refactor.contains("Tcp | Unix"),
            "{}",
            c.possible_refactor
        );
    }

    #[test]
    fn type_only_cluster_of_one_call_bodies_is_not_emitted() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for f in &mut facts[..2] {
            // Drop the `push` call: only the shared helper call is left.
            f.pdg.as_mut().expect("pdg").nodes[3] = node(NodeKind::Param);
        }
        assert!(run(&facts).candidates.is_empty());
    }

    fn guarded() -> Params {
        Params {
            single_call_guard: true,
            ..Params::default()
        }
    }

    /// Dog/Cat corpus where each body keeps one call, fed by `input`.
    fn one_call_bodies(input: NodeKind) -> Vec<FnFacts> {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for f in &mut facts[..2] {
            let pdg = f.pdg.as_mut().expect("pdg");
            pdg.nodes[3] = node(NodeKind::Param);
            pdg.nodes[2] = node(input);
        }
        facts
    }

    #[test]
    fn guard_emits_one_call_bodies_that_do_real_work() {
        let facts = one_call_bodies(NodeKind::Iterate);
        let groups = mine_groups(&facts);
        assert!(
            mine(&facts, &groups, &Params::default())
                .candidates
                .is_empty()
        );
        assert_eq!(mine(&facts, &groups, &guarded()).candidates.len(), 1);
    }

    #[test]
    fn guard_still_rejects_pure_parameter_forwards() {
        let facts = one_call_bodies(NodeKind::Param);
        let mined = mine(&facts, &mine_groups(&facts), &guarded());
        assert!(mined.candidates.is_empty());
    }

    #[test]
    fn type_only_cluster_with_literal_hole_is_not_a_generic_fn() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        for (f, text) in facts[..2].iter_mut().zip(["alpha", "beta"]) {
            let lit = PdgNode {
                kind: NodeKind::Lit,
                lit_kind: Some("str".into()),
                detail: text.into(),
                ..node(NodeKind::Lit)
            };
            let pdg = f.pdg.as_mut().expect("pdg");
            pdg.nodes.push(lit);
            let last = pdg.nodes.len() - 1;
            pdg.edges.push(data(last, 3, 2));
        }
        assert!(no_generic_fn(&facts));
    }

    #[test]
    fn workspace_callee_hole_is_not_a_generic_fn() {
        let mut facts = corpus();
        facts.push(getter("Cow", "moo"));
        for f in &mut facts[..2] {
            f.sig.as_mut().expect("sig").receiver = Receiver::None;
        }
        let kinds: Vec<_> = run(&facts).candidates.iter().map(|c| c.kind).collect();
        assert!(!kinds.contains(&CandidateKind::GenericFn), "{kinds:?}");
    }

    #[test]
    fn type_hole_on_external_types_is_not_a_generic_fn() {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        facts.truncate(2);
        assert!(no_generic_fn(&facts));
    }

    fn existing(trait_method: &str, owners: &[&str]) -> Existing {
        Existing {
            reason: String::new(),
            trait_method: trait_method.into(),
            owners: owners.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn extends_the_trait_implemented_by_the_same_owners_not_the_last_one() {
        let facts = [getter("Dog", "bark"), getter("Cat", "meow")];
        let groups = mine_groups(&facts);
        let defs: Vec<&FnFacts> = facts.iter().collect();
        let verdicts: Verdicts<'_> = vec![(&groups[0], defs)];
        let hidden = [
            existing("k::Speak::speak", &["k::Dog", "k::Cat"]),
            existing("k::Key::public", &["k::Ec", "k::Rsa"]),
        ];
        assert_eq!(
            existing_trait_for(&verdicts, &hidden).as_deref(),
            Some("k::Speak::speak")
        );
        assert_eq!(existing_trait_for(&verdicts, &hidden[1..]), None);
    }

    #[test]
    fn constructor_columns_never_become_trait_candidates() {
        let mut facts = corpus();
        for f in &mut facts[..2] {
            f.sig.as_mut().expect("sig").receiver = Receiver::None;
        }
        let kinds: Vec<_> = run(&facts).candidates.iter().map(|c| c.kind).collect();
        assert!(!kinds.contains(&CandidateKind::TraitMethod), "{kinds:?}");
        assert!(!kinds.contains(&CandidateKind::CapabilitySet), "{kinds:?}");
    }

    #[test]
    fn all_impls_of_same_trait_method_suppressed() {
        let mut facts = corpus();
        for f in &mut facts[..2] {
            f.implements = Some("k::Speak::speak".into());
        }
        let mined = run(&facts);
        assert!(mined.candidates.is_empty());
        assert_eq!(mined.suppressed.len(), 1);
        assert!(mined.suppressed[0].reason.contains("k::Speak::speak"));
    }

    #[test]
    fn effect_mismatch_lowers_score() {
        let clean = run(&corpus());
        let mut facts = corpus();
        facts[0].mutations.push(MutationEvent {
            origin: crate::facts::MutationOrigin::Source,
            root: "self".into(),
            field_path: "n".into(),
            line: 1,
            escapes: true,
            delegated: false,
        });
        let dirty = run(&facts);
        assert!(dirty.candidates[0].score_milli < clean.candidates[0].score_milli);
        assert!(
            dirty.candidates[0]
                .counter_evidence
                .iter()
                .any(|c| c.contains("effect mismatch"))
        );
    }

    #[test]
    fn provenance_lists_every_site() {
        let mut facts = corpus();
        facts.push(getter("Cow", "moo"));
        facts.push(caller("Cow", "moo"));
        let c = &run(&facts).candidates[0];
        assert_eq!(c.sites.len(), 3);
        assert!(
            c.sites
                .iter()
                .all(|s| s.line > 0 && s.path == "src/emit.rs")
        );
        assert_eq!(c.definitions.len(), 3);
        assert_eq!(c.breakdown.support, 3);
    }

    #[test]
    fn size_ratio_filter_applied_before_wl() {
        let mut facts = corpus();
        let mut big = caller("Cow", "moo");
        let extra: Vec<_> = (0..12).map(|_| push_call()).collect();
        big.pdg.as_mut().expect("pdg").nodes.extend(extra);
        facts.push(getter("Cow", "moo"));
        facts.push(big);
        let c = &run(&facts).candidates[0];
        assert_eq!(c.sites.len(), 2, "the oversized look-alike must not join");
    }

    #[test]
    fn common_signature_scores_below_rare_one() {
        let others = (0..12).map(|i| FnFacts {
            sig: Some(method(
                "k::Solo",
                Receiver::Ref,
                vec![],
                TyShape::Prim(format!("p{i}")),
            )),
            ..sample(&format!("solo{i}"))
        });
        let mut rare_facts = corpus();
        rare_facts.extend(others.clone());
        let mut crowd: Vec<FnFacts> = (0..40).map(|i| getter(&format!("T{i}"), "label")).collect();
        crowd.extend(rare_facts.clone());
        let common = &run(&crowd).candidates[0];
        let rare = &run(&rare_facts).candidates[0];
        assert!(common.score_milli < rare.score_milli);
        assert!(common.breakdown.lift_milli < 500);
    }
}
