//! Describe when a reached Go communication completes and the source of a sent
//! value. A channel's capacity alone does not determine its current occupancy.
use super::{Derived, Flow};
use crate::cpg::model::{AccessMode, Function, Node};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Communication {
    pub operation: String,
    pub channel: String,
    pub element: String,
    pub completion: String,
    pub on_close: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_value_source: Option<String>,
}

struct Frame<'a, 'b> {
    body: Body<'a>,
    types: &'b Value,
    reads: HashMap<&'a str, &'a str>,
}

impl Frame<'_, '_> {
    fn fact(&self, name: &str, site: &str) -> Option<&Value> {
        self.body.function.extensions.as_ref()?.get(name)?.get(site)
    }

    fn event(&self, n: &Node) -> Option<(bool, Communication)> {
        let (send, ch, val) = match (n.operator.as_deref()?, &n.children[..]) {
            ("go:receive", [ch]) => (false, ch, None),
            ("go:send", [ch, val]) => (true, ch, Some(val.clone())),
            _ => return None,
        };
        let ty = self.types.get(self.body.node(ch).ok()?.ty.as_deref()?)?;
        let dir = ty.get("direction")?.as_str()?;
        if (send && dir == "receive") || (!send && dir == "send") {
            return None;
        }
        let fact = self.fact("go:channel-operands", &n.id);
        let binding = self.reads.get(ch.as_str()).copied();
        let saved = fact.filter(|f| f.get("binding").and_then(Value::as_str) == binding);
        let capacity = saved
            .and_then(|r| r.get("capacity"))
            .and_then(Value::as_u64);
        let closed = saved.and_then(|r| r.get("closed")).and_then(Value::as_bool) == Some(true);
        Some((
            closed,
            Communication {
                channel: ch.clone(),
                element: ty.get("element")?.as_str()?.into(),
                operation: if send { "send" } else { "receive" }.into(),
                sent_value: val,
                queued_value_source: None,
                binding: binding.map(str::to_owned),
                capacity,
                completion: String::new(),
                on_close: if send { "panic" } else { "buffer_then_zero" }.into(),
            },
        ))
    }

    fn completion(
        &self,
        n: &Node,
        ch: (&Communication, bool),
        flow: (&Flow<'_>, &Derived),
    ) -> &'static str {
        let (event, closed) = ch;
        let (graph, own) = flow;
        let availability = self
            .fact("go:channel-availability", &n.id)
            .and_then(Value::as_str);
        if own.permanent_blocks.contains_key(&n.id)
            && graph
                .vertex(&n.id)
                .is_some_and(|v| graph.edges[v].is_empty())
        {
            return "permanent_block";
        }
        let send = event.sent_value.is_some();
        if closed && !send {
            return "immediate_closed_receive";
        }
        if own
            .panic_paths
            .get(&n.id)
            .is_some_and(|p| p.cause == "send_closed_channel")
        {
            return "immediate_panic";
        }
        if availability == Some("ready_buffered_send") && send {
            return "immediate_buffered_send";
        }
        match (send, event.capacity) {
            (true, Some(0)) => "waiting_receiver_or_closed",
            (false, Some(0)) => "waiting_sender_or_closed",
            (true, _) => "buffer_space_or_waiting_receiver_or_closed",
            (false, _) => "buffered_value_or_waiting_sender_or_closed",
        }
    }
}

pub(crate) fn attach(tables: &Tables<'_>, f: &Function, derived: &mut Derived) {
    let Some(types) = tables.extension("go:channel-types") else {
        return;
    };
    if !f
        .nodes
        .iter()
        .any(|n| matches!(n.operator.as_deref(), Some("go:send" | "go:receive")))
        && !f.refs.iter().any(|r| {
            r.dispatch == Some(crate::cpg::model::Dispatch::Builtin)
                && tables
                    .named_definition(r.target.as_ref().and_then(|t| t.id.as_deref()))
                    .is_some_and(|d| matches!(d.operation.as_deref(), Some("go:len" | "go:cap")))
        })
    {
        return;
    }
    let Ok(body) = Body::new(f) else { return };
    let frame = Frame {
        body,
        types,
        reads: f
            .accesses
            .iter()
            .filter(|a| a.mode == AccessMode::Read)
            .map(|a| (a.node.as_str(), a.symbol.as_str()))
            .collect(),
    };
    super::each_reached(f, derived, |graph, reached, own| {
        for n in f.nodes.iter().filter(|n| reached.contains(n.id.as_str())) {
            if let Some((closed, mut comm)) = frame.event(n) {
                comm.completion = frame.completion(n, (&comm, closed), (graph, own)).into();
                own.communications.insert(n.id.clone(), comm);
            } else if let Some(row) = observations(tables, &frame, n) {
                own.channel_observations.insert(n.id.clone(), row);
            }
        }
        super::channel_buffers::attach(f, graph, own);
    });
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub channel: String,
    pub operation: String,
    pub snapshot: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

fn observations(tables: &Tables<'_>, frame: &Frame<'_, '_>, n: &Node) -> Option<Observation> {
    let r = frame.body.reference(&n.id)?;
    if r.dispatch != Some(crate::cpg::model::Dispatch::Builtin) {
        return None;
    }
    let def = tables.named_definition(r.target.as_ref()?.id.as_deref())?;
    let name = match def.operation.as_deref()? {
        "go:len" => "current_buffered_count",
        "go:cap" => "fixed_capacity",
        _ => return None,
    };
    let [_, channel] = n.children.as_slice() else {
        return None;
    };
    frame
        .types
        .get(frame.body.node(channel).ok()?.ty.as_deref()?)?;
    Some(Observation {
        channel: channel.clone(),
        operation: def.operation.clone()?,
        snapshot: name.into(),
        binding: frame.reads.get(channel.as_str()).map(|s| (*s).to_owned()),
    })
}
