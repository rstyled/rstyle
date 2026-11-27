//! Attribute forced exceptional transfers to their source, then follow unwinding
//! inside each execution frame. Dynamic dispatch and unknown callees stay opaque.
use super::{Derived, Flow, Vertex};
use crate::cpg::model::{Dispatch, Exit, Function, Ref};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanicPath {
    pub cause: String,
    pub origin_function: String,
    pub origin_node: String,
    pub payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument: Option<String>,
    pub defers: BTreeSet<String>,
    pub callee_defers: Vec<String>,
    /// Candidate direct recovery sites in the first applicable deferred call.
    pub direct_recovers: BTreeMap<String, String>,
    pub exits: BTreeMap<String, String>,
    pub unhandled_program_exit: bool,
    /// The runtime reports this panic payload when it escapes the top-level goroutine.
    pub report_payload: bool,
}

fn fact<'a>(f: &'a Function, key: &str) -> Option<&'a Value> {
    f.extensions.as_ref()?.get(key)
}

fn direct(f: &Function, id: &str) -> Option<PanicPath> {
    let v = fact(f, "go:panic-causes")?.get(id)?;
    let cause = v.get("kind")?.as_str()?;
    let payload = match cause {
        "explicit" if v.get("nil_interface").and_then(Value::as_bool) == Some(true) => {
            "panic_nil_error"
        }
        "explicit" => "argument",
        _ => "runtime_error",
    };
    Some(PanicPath {
        cause: cause.into(),
        origin_function: f.id.clone(),
        origin_node: id.into(),
        payload: payload.into(),
        argument: v.get("argument").and_then(Value::as_str).map(str::to_owned),
        defers: BTreeSet::new(),
        callee_defers: Vec::new(),
        direct_recovers: BTreeMap::new(),
        exits: BTreeMap::new(),
        unhandled_program_exit: false,
        report_payload: false,
    })
}

fn callee<'a>(tables: &Tables<'a>, r: &Ref) -> Option<&'a Function> {
    if r.dispatch != Some(Dispatch::Static) {
        return None;
    }
    tables.function(r.target.as_ref()?.id.as_deref()?)
}

fn forced(g: &Flow<'_>, at: Vertex) -> bool {
    !g.edges[at].is_empty() && g.edges[at].iter().all(|(_, kind, _)| *kind == "exception")
}

// A static wrapper with one forced source and nowhere to return normally.
fn escaping(tables: &Tables<'_>, f: &Function) -> Option<PanicPath> {
    let causes = fact(f, "go:panic-causes")?.as_object()?;
    let graph = Flow::of(f)?;
    let reached = graph.reached();
    let mut exits = f
        .cfg
        .exits
        .iter()
        .filter(|x| reached.contains(x.id.as_str()));
    let exit = exits.next()?;
    if exits.next().is_some() || exit.kind != "exceptional" {
        return None;
    }
    let mut sources = causes.keys().filter(|s| reached.contains(s.as_str()));
    let src = sources.next()?;
    let at = graph.vertex(src)?;
    if sources.next().is_some() || !forced(&graph, at) {
        return None;
    }
    let body = Body::new(f).ok()?;
    let mut path = direct(f, src)?;
    let mut cursor = at;
    let mut seen = HashSet::new();
    loop {
        let [next] = graph.forward.successors[cursor].as_slice() else {
            return None;
        };
        if graph.id(*next) == exit.id {
            return Some(path);
        }
        let id = graph.id(*next);
        if !seen.insert(id) || !inert_print(tables, body.reference(id)?) {
            return None;
        }
        path.callee_defers.push(id.into());
        cursor = *next;
    }
}

fn inert_print(tables: &Tables<'_>, r: &Ref) -> bool {
    r.dispatch == Some(Dispatch::Builtin)
        && tables
            .named_definition(r.target.as_ref().and_then(|t| t.id.as_deref()))
            .is_some_and(|d| matches!(d.operation.as_deref(), Some("go:print" | "go:println")))
}

fn recovers(f: &Function, graph: &Flow<'_>) -> Vec<String> {
    let Some(sites) = fact(f, "go:recover-sites").and_then(Value::as_array) else {
        return Vec::new();
    };
    let reached = graph.reached();
    sites
        .iter()
        .filter_map(Value::as_str)
        .filter(|id| reached.contains(id))
        .map(str::to_owned)
        .collect()
}

fn handler(tables: &Tables<'_>, f: &Function, r: &Ref) -> Option<Vec<String>> {
    if let Some(id) = &r.closure {
        let closure = f.cfg.closures.iter().find(|c| c.node == *id)?;
        return Some(recovers(f, &Flow::of_closure(closure)?));
    }
    let target = callee(tables, r)?;
    Some(recovers(target, &Flow::of(target)?))
}

fn program_main(tables: &Tables<'_>, f: &Function) -> bool {
    f.name == "main"
        && f.signature.receiver.is_none()
        && tables
            .extension("go:package-clauses")
            .and_then(|m| m.get(&f.span.file))
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            == Some("main")
}

struct Frame<'a> {
    tables: &'a Tables<'a>,
    function: &'a Function,
    body: Body<'a>,
    main: bool,
}

impl Frame<'_> {
    fn fill_path(
        &self,
        flow: (&Flow<'_>, &[Exit]),
        start: (Vertex, &Derived),
        path: &mut PanicPath,
    ) {
        let (graph, exits) = flow;
        let (at, own) = start;
        let defers: BTreeMap<_, _> = own
            .scheduled_calls
            .values()
            .filter(|s| s.mode == "defer")
            .map(|s| (s.call.as_str(), s))
            .collect();
        let terminals: BTreeMap<_, _> = exits
            .iter()
            .map(|x| (x.id.as_str(), x.kind.as_str()))
            .collect();
        let mut pending: Vec<_> = graph.forward.successors[at]
            .iter()
            .map(|&v| (v, true))
            .collect();
        let mut seen = HashSet::new();
        while let Some((v, mut active)) = pending.pop() {
            if !seen.insert((v, active)) {
                continue;
            }
            let id = graph.id(v);
            if let Some(kind) = terminals.get(id) {
                path.exits.insert(id.into(), (*kind).into());
                path.unhandled_program_exit |=
                    self.main && graph.closure.is_none() && *kind == "exceptional";
                path.report_payload |= path.unhandled_program_exit && active;
            }
            if defers.contains_key(id) {
                path.defers.insert(id.into());
                if active {
                    let direct = self
                        .body
                        .reference(id)
                        .and_then(|r| handler(self.tables, self.function, r));
                    if let Some(sites) = &direct
                        && let [single] = sites.as_slice()
                    {
                        path.direct_recovers.insert(id.into(), single.clone());
                    }
                    // A handler may consume this panic or start another. Avoid carrying
                    // the same payload through an unmodelled deferred invocation.
                    active = self
                        .body
                        .reference(id)
                        .is_some_and(|r| inert_print(self.tables, r));
                }
            }
            pending.extend(graph.forward.successors[v].iter().map(|&to| (to, active)));
        }
    }

    fn boundaries(
        &self,
        boundary: (&Flow<'_>, &[Exit]),
        reached: &HashSet<&str>,
        own: &mut Derived,
    ) {
        let (graph, exits) = boundary;
        if self.main && graph.closure.is_none() {
            for e in exits
                .iter()
                .filter(|e| e.kind == "normal" && reached.contains(e.id.as_str()))
            {
                own.program_return_exits
                    .insert(e.id.clone(), "exit_without_waiting_for_goroutines".into());
            }
        }
        for (site, registered) in &own.scheduled_calls {
            if registered.mode == "goroutine"
                && let Some(mut path) = self
                    .body
                    .reference(&registered.call)
                    .and_then(|r| callee(self.tables, r))
                    .and_then(|f| escaping(self.tables, f))
            {
                path.unhandled_program_exit = true;
                path.report_payload = true;
                own.goroutine_panics.insert(site.clone(), path);
            }
        }
    }

    fn attach(&self, graph: &Flow<'_>, reached: HashSet<&str>, own: &mut Derived) {
        let exits = graph
            .closure
            .and_then(|id| {
                self.function
                    .cfg
                    .closures
                    .iter()
                    .find(|c| c.node == id)
                    .map(|c| c.exits.as_slice())
            })
            .unwrap_or(&self.function.cfg.exits);
        self.boundaries((graph, exits), &reached, own);
        let scheduled: HashSet<_> = own
            .scheduled_calls
            .values()
            .map(|s| s.call.clone())
            .collect();
        for id in reached {
            let Some(at) = graph.vertex(id) else { continue };
            let target = self.body.reference(id).and_then(|r| callee(self.tables, r));
            if !scheduled.contains(id)
                && let Some(t) = target
                && fact(t, "go:recover-sites").is_some()
                && let Some(g) = Flow::of(t)
            {
                let sites = recovers(t, &g);
                if !sites.is_empty() {
                    own.ordinary_recovers.insert(id.into(), sites);
                }
            }
            if !forced(graph, at) {
                continue;
            }
            if let Some(mut path) =
                direct(self.function, id).or_else(|| escaping(self.tables, target?))
            {
                self.fill_path((graph, exits), (at, own), &mut path);
                own.panic_paths.insert(id.into(), path);
            }
        }
    }
}

pub(crate) fn attach(tables: &Tables<'_>, f: &Function, derived: &mut Derived) {
    if tables.language() != "go"
        || !(program_main(tables, f)
            || fact(f, "go:panic-causes").is_some()
            || f.refs.iter().any(|r| {
                callee(tables, r).is_some_and(|t| {
                    fact(t, "go:panic-causes").is_some() || fact(t, "go:recover-sites").is_some()
                })
            }))
    {
        return;
    }
    let Ok(body) = Body::new(f) else { return };
    let frame = Frame {
        tables,
        function: f,
        body,
        main: program_main(tables, f),
    };
    super::each_reached(f, derived, |g, r, own| frame.attach(g, r, own));
}
