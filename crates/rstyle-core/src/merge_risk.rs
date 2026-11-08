//! What changed between two runs of a codebase, read from their documents, so every language gets
//! it without code of its own. Functions correspond by locator, which survives what renumbers a
//! stable identity. A text change with no change in the compared facts says only that those facts
//! didn't change, not that behavior didn't.

mod commits;
pub mod rating;

use crate::config::Config;
use crate::cpg::model::Document;
use crate::cpg::records::{DeclaredSignature, Measured, Record};
use crate::facts::{FnFacts, NodeKind, Pdg, PdgNode, TyShape};
use crate::{exclusions, graph, locator, rules};
pub use commits::{ChangedPath, Commits, Unread};
use rating::{Measured as Evidence, Rating};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::sync::Mutex;

/// The source files a run's documents hold, by path, seen as they're read. A run's records only
/// say what's in its functions, and a file with no functions has no records at all.
#[derive(Default)]
pub struct SourceFiles(Mutex<HeldFiles>);

#[derive(Default)]
struct HeldFiles {
    /// Each written file's digest.
    written: BTreeMap<String, String>,
    /// The files a frontend marks as tests, which aren't what ships.
    tests: BTreeSet<String>,
}

impl SourceFiles {
    /// Keeps what the document says of its written files and its tests. A generated file has no
    /// author to tell, and a path that climbs out of the root isn't this code's own.
    pub fn see(&self, document: &Document) {
        let Some(unit) = document.unit.as_ref() else {
            return;
        };
        let mut held = self.0.lock().expect("no panic while keeping");
        let own = unit
            .files
            .iter()
            .filter(|file| file.origin == "source")
            .filter(|file| !file.path.starts_with("../") && !Path::new(&file.path).is_absolute());
        for file in own {
            if file.test {
                held.tests.insert(file.path.clone());
                continue;
            }
            held.written
                .entry(file.path.clone())
                .or_insert_with(|| file.digest.clone());
        }
    }

    fn selected(self, excluded: &exclusions::Matcher) -> HeldFiles {
        let mut held = self.0.into_inner().expect("no panic while keeping");
        let kept = |path: &String| !excluded.is_excluded(Path::new(path));

        held.written.retain(|path, _| kept(path));
        held.tests.retain(kept);
        held
    }
}

/// What a function declares and whether its tree is all of it, beside its facts.
#[derive(Clone)]
struct Declared {
    signature: DeclaredSignature,
    shapes: Vec<Option<TyShape>>,
    values: Vec<String>,
    gap: Option<&'static str>,
}

/// One run, as a comparison reads it.
#[derive(Default)]
pub struct Snapshot {
    facts: Vec<FnFacts>,
    /// Beside `facts`.
    declared: Vec<Declared>,
    index: BTreeMap<String, usize>,
    /// Each test function's text, by identity. Changing a test isn't a risk, but it's worth saying.
    tests: BTreeMap<String, String>,
    /// The text of functions with nothing to compare, what a macro or a build writes, by identity.
    not_compared: BTreeMap<String, String>,
    /// Each written file's digest, by path.
    files: BTreeMap<String, String>,
    /// The files a frontend marks as tests.
    test_files: BTreeSet<String>,
}

impl Snapshot {
    /// Reads a run's records in canonical order. A function several compilations hold is read once.
    pub fn of(records: Vec<Record>, files: SourceFiles, config: &Config) -> Result<Self, String> {
        let excluded = exclusions::Matcher::compile(&config.exclude.0)?;
        let held = files.selected(&excluded);
        let mut snapshot = Self {
            files: held.written,
            test_files: held.tests,
            ..Self::default()
        };

        for record in records {
            if !excluded.is_excluded(Path::new(&record.path)) {
                snapshot.add(record);
            }
        }
        Ok(snapshot.corresponded())
    }

    /// Names each function, and each callee its graph names, by locator: adding one impl block
    /// renumbers every later one's methods, which would read as all of them and their callers
    /// changing.
    fn corresponded(mut self) -> Self {
        let locators = locator::assign(&self.facts);
        for facts in &mut self.facts {
            name_callees_by_locator(facts, &locators);
        }

        self.index = self
            .facts
            .iter()
            .enumerate()
            .filter_map(|(at, facts)| Some((locators.get(&facts.id)?.clone(), at)))
            .collect();
        self
    }

    fn add(&mut self, record: Record) {
        if record.test {
            self.tests.entry(record.identity).or_insert(record.source);
            return;
        }
        let Measured::Known(mut facts) = record.measured else {
            self.not_compared
                .entry(record.identity)
                .or_insert(record.source);
            return;
        };

        facts.pdg = record.graph;
        self.add_function(
            *facts,
            Declared {
                signature: record.signature,
                shapes: record.signature_shapes,
                values: record.known_values,
                gap: record.syntax_gap,
            },
        );
    }

    fn add_function(&mut self, facts: FnFacts, declared: Declared) {
        if !self.index.contains_key(&facts.id) {
            self.index.insert(facts.id.clone(), self.facts.len());
            self.facts.push(facts);
            self.declared.push(declared);
        }
    }

    fn function(&self, identity: &str) -> Option<(&FnFacts, &Declared)> {
        let &at = self.index.get(identity)?;
        Some((&self.facts[at], &self.declared[at]))
    }

    /// The written files its documents hold, by path from the source.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }
}

fn name_callees_by_locator(facts: &mut FnFacts, locators: &BTreeMap<String, String>) {
    let nodes = facts.pdg.iter_mut().flat_map(|graph| &mut graph.nodes);
    for node in nodes {
        let located = node
            .callee_id
            .as_ref()
            .and_then(|callee| locators.get(callee));
        if let Some(located) = located {
            node.callee_id = Some(located.clone());
        }
    }
}

/// What kind of change a function saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Its tree isn't all of it, so nothing can be said of what changed.
    Unknown,
    /// What it depends on and does is different.
    Changed,
    Removed,
    Added,
    /// Its text is different, and none of the compared facts are.
    TextOnly,
}

impl ChangeKind {
    fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Changed => "changed",
            Self::Removed => "removed",
            Self::Added => "added",
            Self::TextOnly => "text only",
        }
    }
}

/// How far a function's dependence graph moved, line numbers aside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GraphDelta {
    pub nodes_added: usize,
    pub nodes_removed: usize,
    pub edges_added: usize,
    pub edges_removed: usize,
}

impl GraphDelta {
    fn is_empty(self) -> bool {
        self == Self::default()
    }
}

/// An effect a function gained or lost.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct EffectChange {
    pub kind: &'static str,
    pub label: String,
}

/// What changed of one function, and what reaches it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionChange {
    pub function: String,
    pub name: String,
    pub path: String,
    pub line: usize,
    pub kind: ChangeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    pub signature_changed: bool,
    /// A value the frontend knows at one of its nodes changed, as a constant's does at a use.
    pub values_changed: bool,
    pub graph: GraphDelta,
    pub calls_added: Vec<String>,
    pub calls_removed: Vec<String>,
    /// It makes a call whose target can't be known, in either run, so nothing can be said of all it
    /// calls.
    pub unresolved_calls: bool,
    /// Library callees it calls more often that nothing says the effect of, by identity.
    pub calls_unclassified: Vec<String>,
    pub effects_gained: Vec<EffectChange>,
    pub effects_lost: Vec<EffectChange>,
    /// Functions that call it directly, tests aside.
    pub callers: usize,
    /// Functions that reach it through any chain of calls, tests aside.
    pub reached_by: usize,
}

/// What changed between two runs.
#[derive(Debug, Serialize)]
pub struct ChangeReport {
    pub schema_version: u32,
    #[serde(flatten)]
    pub rating: Rating,
    pub changes: Vec<FunctionChange>,
    pub unchanged: usize,
    pub tests_changed: usize,
    /// Functions a macro or a build writes, which aren't compared.
    pub not_compared: usize,
    /// How many of those differ between the runs.
    pub not_compared_changed: usize,
    /// Written files whose text changed with no function in them changing: a constant, a
    /// package-level value or a type's fields can change what a program does, and no fact here says
    /// how, so these need a person.
    pub changed_outside_functions: Vec<String>,
    /// Changed files nothing here can see into, by the runs' commits: what no document holds, such
    /// as scripts, workflows, build files, lockfiles and configuration. `None` when the commits
    /// couldn't be compared, and a reason says why.
    pub changed_files_unread: Option<Vec<Unread>>,
    /// Changed files the base's policy excludes, which aren't looked at.
    pub changed_files_excluded: Vec<String>,
    /// Changed files a frontend marks as tests, which aren't rated, as the functions in them aren't.
    pub changed_files_tests: Vec<String>,
}

/// Each function's direct callers, and everything that reaches it through them.
struct Reach {
    callers: Vec<Vec<usize>>,
}

impl Reach {
    fn of(snapshot: &Snapshot) -> Self {
        Self {
            callers: graph::build(&snapshot.facts).callers(),
        }
    }

    /// Direct callers, then every function a chain of calls reaches this one from.
    fn counts(&self, at: usize) -> (usize, usize) {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([at]);
        while let Some(function) = queue.pop_front() {
            for &caller in &self.callers[function] {
                if caller != at && seen.insert(caller) {
                    queue.push_back(caller);
                }
            }
        }
        (self.callers[at].len(), seen.len())
    }
}

/// A node as it compares: everything it says but its line and how its callee is shaped, which its
/// callee's identity already says.
fn same_node(a: &PdgNode, b: &PdgNode) -> bool {
    a.kind == b.kind
        && a.ty == b.ty
        && a.callee_id == b.callee_id
        && a.macro_name == b.macro_name
        && a.lit_kind == b.lit_kind
        && a.detail == b.detail
}

/// Each node's key, by what it says and how many like it came before, so two runs agree on which
/// is which whatever line they're on. Reordering nodes that say the same reads as a change.
fn node_keys(graph: &Pdg) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    graph
        .nodes
        .iter()
        .map(|node| {
            let label = format!(
                "{:?} {:?} {:?} {:?} {:?} {}",
                node.kind, node.ty, node.callee_id, node.macro_name, node.lit_kind, node.detail
            );
            let count = seen.entry(label.clone()).or_default();
            *count += 1;
            format!("{label} #{count}")
        })
        .collect()
}

fn edge_keys(graph: &Pdg, nodes: &[String]) -> BTreeSet<String> {
    graph
        .edges
        .iter()
        .map(|edge| {
            format!(
                "{} {:?} {} {}",
                nodes[edge.from], edge.kind, edge.arg_pos, nodes[edge.to]
            )
        })
        .collect()
}

fn graph_delta(base: &Pdg, head: &Pdg) -> GraphDelta {
    let same_nodes = base.nodes.len() == head.nodes.len()
        && base
            .nodes
            .iter()
            .zip(&head.nodes)
            .all(|(a, b)| same_node(a, b));
    if same_nodes && base.edges == head.edges {
        return GraphDelta::default();
    }

    let (before, after) = (node_keys(base), node_keys(head));
    let (nodes_before, nodes_after): (BTreeSet<_>, BTreeSet<_>) =
        (before.iter().collect(), after.iter().collect());
    let (edges_before, edges_after) = (edge_keys(base, &before), edge_keys(head, &after));
    GraphDelta {
        nodes_added: nodes_after.difference(&nodes_before).count(),
        nodes_removed: nodes_before.difference(&nodes_after).count(),
        edges_added: edges_after.difference(&edges_before).count(),
        edges_removed: edges_before.difference(&edges_after).count(),
    }
}

/// How many times a function calls each callee it names.
fn calls_of(graph: Option<&Pdg>) -> BTreeMap<&str, usize> {
    let mut found = BTreeMap::new();
    let nodes = graph.into_iter().flat_map(|graph| &graph.nodes);
    for node in nodes.filter(|node| node.kind == NodeKind::Call) {
        if let Some(callee) = node.callee_id.as_deref() {
            *found.entry(callee).or_default() += 1;
        }
    }
    found
}

/// The callees called more often in `after` than `before`, sorted.
fn more_calls(before: &BTreeMap<&str, usize>, after: &BTreeMap<&str, usize>) -> Vec<String> {
    after
        .iter()
        .filter(|(callee, count)| before.get(*callee).is_none_or(|was| was < count))
        .map(|(callee, _)| (*callee).to_owned())
        .collect()
}

/// How many times a function calls each library callee nothing says the effect of.
fn unclassified_calls<'f>(facts: &'f FnFacts, config: &Config) -> BTreeMap<&'f str, usize> {
    let mut found = BTreeMap::new();
    for call in &facts.calls {
        if let Some(callee) = rules::unclassified_callee(call, config) {
            *found.entry(callee).or_default() += 1;
        }
    }
    found
}

fn has_unresolved_calls(facts: &FnFacts) -> bool {
    facts.calls.iter().any(|call| call.dynamic)
}

fn effects_of(facts: &FnFacts, config: &Config) -> BTreeSet<EffectChange> {
    rules::effects_of(facts, config)
        .into_iter()
        .map(|effect| EffectChange {
            kind: effect.kind.name(),
            label: effect.label,
        })
        .collect()
}

fn gained(from: &BTreeSet<EffectChange>, to: &BTreeSet<EffectChange>) -> Vec<EffectChange> {
    to.difference(from).cloned().collect()
}

/// Whether two functions declare the same: the same types, by what they are, written the same
/// way. A shape alone can't tell two maps apart, and a frontend may have nothing written to read.
fn same_signature(a: &Declared, b: &Declared) -> bool {
    let (x, y) = (&a.signature, &b.signature);
    a.shapes == b.shapes
        && x.has_self == y.has_self
        && x.generics == y.generics
        && x.modifiers == y.modifiers
        && x.result.spelling == y.result.spelling
        && x.parameters.len() == y.parameters.len()
        && x.parameters
            .iter()
            .zip(&y.parameters)
            .all(|(x, y)| x.spelling == y.spelling)
}

/// What two runs' facts of one function say changed, or nothing when they agree.
fn compared(
    (before, declared_before): (&FnFacts, &Declared),
    (after, declared_after): (&FnFacts, &Declared),
    config: &Config,
) -> Option<FunctionChange> {
    let delta = match (&before.pdg, &after.pdg) {
        (Some(base), Some(head)) => graph_delta(base, head),
        _ => GraphDelta::default(),
    };
    let text_changed = before.source != after.source;
    let values_changed = declared_before.values != declared_after.values;
    if !text_changed && !values_changed && delta.is_empty() {
        return None;
    }

    let reason = declared_before.gap.or(declared_after.gap);
    let kind = match (reason, !values_changed && delta.is_empty()) {
        (Some(_), _) => ChangeKind::Unknown,
        (None, true) => ChangeKind::TextOnly,
        (None, false) => ChangeKind::Changed,
    };
    let (calls_before, calls_after) = (calls_of(before.pdg.as_ref()), calls_of(after.pdg.as_ref()));
    let (effects_before, effects_after) = (effects_of(before, config), effects_of(after, config));
    Some(FunctionChange {
        reason,
        signature_changed: !same_signature(declared_before, declared_after),
        values_changed,
        graph: delta,
        calls_added: more_calls(&calls_before, &calls_after),
        calls_removed: more_calls(&calls_after, &calls_before),
        unresolved_calls: has_unresolved_calls(before) || has_unresolved_calls(after),
        calls_unclassified: more_calls(
            &unclassified_calls(before, config),
            &unclassified_calls(after, config),
        ),
        effects_gained: gained(&effects_before, &effects_after),
        effects_lost: gained(&effects_after, &effects_before),
        ..one_sided(after, kind)
    })
}

fn all_calls(facts: &FnFacts) -> Vec<String> {
    more_calls(&BTreeMap::new(), &calls_of(facts.pdg.as_ref()))
}

/// A function only one run has: a new one gains everything it does, and a removed one loses it.
fn present_once(facts: &FnFacts, kind: ChangeKind, config: &Config) -> FunctionChange {
    let (calls, effects) = (
        all_calls(facts),
        effects_of(facts, config).into_iter().collect(),
    );
    let change = FunctionChange {
        unresolved_calls: has_unresolved_calls(facts),
        ..one_sided(facts, kind)
    };
    match kind {
        ChangeKind::Added => FunctionChange {
            calls_added: calls,
            calls_unclassified: more_calls(&BTreeMap::new(), &unclassified_calls(facts, config)),
            effects_gained: effects,
            ..change
        },
        _ => FunctionChange {
            calls_removed: calls,
            effects_lost: effects,
            ..change
        },
    }
}

/// A change to a function only one run has, or the start of one both have.
fn one_sided(facts: &FnFacts, kind: ChangeKind) -> FunctionChange {
    FunctionChange {
        function: facts.id.clone(),
        name: facts.name.clone(),
        path: facts.path.clone(),
        line: facts.line,
        kind,
        reason: None,
        signature_changed: false,
        values_changed: false,
        graph: GraphDelta::default(),
        calls_added: Vec::new(),
        calls_removed: Vec::new(),
        unresolved_calls: false,
        calls_unclassified: Vec::new(),
        effects_gained: Vec::new(),
        effects_lost: Vec::new(),
        callers: 0,
        reached_by: 0,
    }
}

fn changed_tests(base: &Snapshot, head: &Snapshot) -> usize {
    let identities: BTreeSet<&String> = base.tests.keys().chain(head.tests.keys()).collect();
    identities
        .into_iter()
        .filter(|identity| base.tests.get(*identity) != head.tests.get(*identity))
        .count()
}

/// What changed of each function, most in need of a look first, among `identities`.
fn function_changes(
    identities: &BTreeSet<&String>,
    (base, head): (&Snapshot, &Snapshot),
    config: &Config,
) -> Vec<FunctionChange> {
    let (reach_before, reach_after) = (Reach::of(base), Reach::of(head));

    let mut changes: Vec<FunctionChange> = identities
        .iter()
        .copied()
        .filter_map(|identity| {
            let (mut change, reach, at) = match (base.function(identity), head.function(identity)) {
                (Some(before), Some(after)) => (
                    compared(before, after, config)?,
                    &reach_after,
                    head.index[identity],
                ),
                (None, Some((after, _))) => (
                    present_once(after, ChangeKind::Added, config),
                    &reach_after,
                    head.index[identity],
                ),
                (Some((before, _)), None) => (
                    present_once(before, ChangeKind::Removed, config),
                    &reach_before,
                    base.index[identity],
                ),
                (None, None) => return None,
            };
            (change.callers, change.reached_by) = reach.counts(at);
            Some(change)
        })
        .collect();
    changes.sort_by(|a, b| {
        (a.kind, &a.path, a.line, &a.function).cmp(&(b.kind, &b.path, b.line, &b.function))
    });
    changes
}

/// What changed from `base` to `head`, and how much that matters by `config`'s rules. `commits`
/// says what else changed, which no document holds.
pub fn compare(
    base: &Snapshot,
    head: &Snapshot,
    config: &Config,
    commits: &Commits,
) -> Result<ChangeReport, String> {
    let identities: BTreeSet<&String> = base.index.keys().chain(head.index.keys()).collect();
    let changes = function_changes(&identities, (base, head), config);
    let excluded = exclusions::Matcher::compile(&config.exclude.0)?;
    let held = |path: &str| {
        let written = base.files.contains_key(path) || head.files.contains_key(path);
        let test = base.test_files.contains(path) || head.test_files.contains(path);
        match (written, test) {
            (true, _) => commits::Held::Written,
            (false, true) => commits::Held::Test,
            (false, false) => commits::Held::No,
        }
    };
    let outside = commits::Outside::of(commits, held, &excluded);

    let not_compared: BTreeSet<&String> = base
        .not_compared
        .keys()
        .chain(head.not_compared.keys())
        .collect();
    let not_compared_changed = not_compared_changed(base, head);
    let changed_outside_functions = changed_outside_functions(base, head, &changes);
    let rating = rating::rate(
        &Evidence {
            changes: &changes,
            changed_outside_functions: &changed_outside_functions,
            not_compared_changed,
            unread: outside.unread.as_deref().map_err(String::as_str),
        },
        &config.risk,
    );

    Ok(ChangeReport {
        schema_version: 1,
        rating,
        unchanged: identities.len() - changes.len(),
        tests_changed: changed_tests(base, head),
        not_compared: not_compared.len(),
        not_compared_changed,
        changed_outside_functions,
        changed_files_unread: outside.unread.ok(),
        changed_files_excluded: outside.excluded,
        changed_files_tests: outside.tests,
        changes,
    })
}

/// How many functions with nothing to compare differ between the runs, by what they say: what a
/// macro expands to is numbered by what comes before it, so one more expansion would otherwise read
/// as every later one changing.
fn not_compared_changed(base: &Snapshot, head: &Snapshot) -> usize {
    let by_text = |snapshot: &'_ Snapshot| {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for source in snapshot.not_compared.values() {
            *counts.entry(source.clone()).or_default() += 1;
        }
        counts
    };
    let (before, after) = (by_text(base), by_text(head));

    let texts: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let difference = |text: &&String| {
        let count = |counts: &BTreeMap<String, usize>| counts.get(*text).copied().unwrap_or(0);
        count(&before).abs_diff(count(&after))
    };
    texts.iter().map(difference).sum()
}

/// Written files that differ between the runs, leaving out those a function change already sends a
/// person to.
fn changed_outside_functions(
    base: &Snapshot,
    head: &Snapshot,
    changes: &[FunctionChange],
) -> Vec<String> {
    let reported: BTreeSet<&str> = changes.iter().map(|change| change.path.as_str()).collect();
    let paths: BTreeSet<&String> = base.files.keys().chain(head.files.keys()).collect();
    paths
        .into_iter()
        .filter(|path| base.files.get(*path) != head.files.get(*path))
        .filter(|path| !reported.contains(path.as_str()))
        .cloned()
        .collect()
}

fn effect_lines(verb: &'static str, effects: &[EffectChange]) -> impl Iterator<Item = String> {
    effects
        .iter()
        .map(move |effect| format!("  {verb} {}: {}", effect.kind, effect.label))
}

/// The calls that changed. All of a new or removed function's are what it is, not what changed.
fn call_lines(change: &FunctionChange) -> Vec<String> {
    if matches!(change.kind, ChangeKind::Added | ChangeKind::Removed) {
        return Vec::new();
    }
    let lines = |verb: &'static str, calls: &[String]| {
        let lines = calls.iter().map(|call| format!("  calls {verb}: {call}"));
        lines.collect::<Vec<_>>()
    };
    [
        lines("more", &change.calls_added),
        lines("less", &change.calls_removed),
    ]
    .concat()
}

fn change_lines(change: &FunctionChange) -> Vec<String> {
    let summary = format!(
        "{}:{}: {}: {}, {} callers, reached by {}",
        change.path,
        change.line,
        change.name,
        change.kind.name(),
        change.callers,
        change.reached_by
    );
    let reason = change.reason.map(|reason| format!("  {reason}"));
    let signature = change
        .signature_changed
        .then(|| "  signature changed".to_owned());
    let values = change
        .values_changed
        .then(|| "  a known value changed, as a constant's does".to_owned());
    let unresolved = change
        .unresolved_calls
        .then(|| "  makes a call whose target can't be known".to_owned());
    let unclassified = change
        .calls_unclassified
        .iter()
        .map(|callee| format!("  new library call nothing classifies: {callee}"));

    std::iter::once(summary)
        .chain(reason)
        .chain(signature)
        .chain(values)
        .chain(unresolved)
        .chain(unclassified)
        .chain(effect_lines("gains", &change.effects_gained))
        .chain(effect_lines("loses", &change.effects_lost))
        .chain(call_lines(change))
        .collect()
}

impl ChangeReport {
    fn count(&self, kind: ChangeKind) -> usize {
        self.changes
            .iter()
            .filter(|change| change.kind == kind)
            .count()
    }

    /// Each changed file nothing here can see into, or why they can't be listed, then each test
    /// file, which isn't rated.
    fn unread_lines(&self) -> Vec<String> {
        let unread = match (&self.changed_files_unread, self.rating.unlisted()) {
            (Some(files), _) => files.iter().map(Unread::line).collect(),
            (None, why) => vec![format!(
                "Changed files can't be listed: {}",
                why.unwrap_or("the runs' commits weren't compared")
            )],
        };
        let tests = self
            .changed_files_tests
            .iter()
            .map(|path| format!("{path}: changed, and it's a test file, which isn't rated"));
        unread.into_iter().chain(tests).collect()
    }

    /// How many changed files went where, by the runs' commits.
    fn files_summary(&self) -> String {
        let Some(unread) = &self.changed_files_unread else {
            return "Changed files can't be listed.".to_owned();
        };
        format!(
            "Changed files nothing here can see into: {}, and {} the policy excludes. Test files \
             changed: {}.",
            unread.len(),
            self.changed_files_excluded.len(),
            self.changed_files_tests.len(),
        )
    }

    pub fn text(&self) -> String {
        let summary = format!(
            "Changes: {} changed, {} added, {} removed, {} text only, {} unknown; {} unchanged. \
             Files changed outside any function: {}. {} Test functions changed: {}. Generated \
             functions not compared: {}, {} of them changed.",
            self.count(ChangeKind::Changed),
            self.count(ChangeKind::Added),
            self.count(ChangeKind::Removed),
            self.count(ChangeKind::TextOnly),
            self.count(ChangeKind::Unknown),
            self.unchanged,
            self.changed_outside_functions.len(),
            self.files_summary(),
            self.tests_changed,
            self.not_compared,
            self.not_compared_changed,
        );

        let outside = self
            .changed_outside_functions
            .iter()
            .map(|path| format!("{path}: changed outside any function"));
        let lines = self.changes.iter().flat_map(change_lines);
        self.rating
            .lines(self.changes.len())
            .into_iter()
            .chain(lines)
            .chain(outside)
            .chain(self.unread_lines())
            .chain(std::iter::once(summary))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    }
}

#[cfg(test)]
mod tests;
