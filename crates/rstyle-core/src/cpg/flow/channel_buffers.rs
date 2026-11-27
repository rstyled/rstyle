//! Track the FIFO only while a fresh channel is private to the current frame
//! and its path has no incoming join or outgoing alternative.
use super::{Derived, Flow};
use crate::cpg::model::{AccessMode, Function};
use std::collections::{HashMap, HashSet, VecDeque};

fn private<'a>(f: &'a Function, own: &Derived) -> HashMap<&'a str, VecDeque<String>> {
    let reads: HashSet<&str> = own
        .communications
        .values()
        .filter(|c| c.capacity.is_some())
        .map(|c| c.channel.as_str())
        .collect();
    let mut writes = HashMap::<&str, usize>::new();
    let mut disqualified = HashSet::new();
    for a in &f.accesses {
        match a.mode {
            AccessMode::Write => {
                *writes.entry(&a.symbol).or_default() += 1;
            }
            AccessMode::Read if reads.contains(a.node.as_str()) => (),
            _ => {
                disqualified.insert(a.symbol.as_str());
            }
        }
    }
    writes
        .into_iter()
        .filter(|(sym, n)| *n == 1 && !disqualified.contains(*sym))
        .map(|(s, _)| (s, VecDeque::new()))
        .collect()
}

pub(super) fn attach(f: &Function, graph: &Flow<'_>, own: &mut Derived) {
    if !own
        .communications
        .values()
        .any(|c| c.capacity.is_some_and(|n| n > 0))
    {
        return;
    }
    let mut queues = private(f, own);
    if queues.is_empty() {
        return;
    }
    let mut cursor = graph.entry();
    let mut seen = HashSet::new();
    while seen.insert(cursor) {
        if let Some(row) = own.communications.get_mut(graph.id(cursor))
            && let Some(q) = row.binding.as_deref().and_then(|s| queues.get_mut(s))
            && let Some(capacity) = row.capacity
        {
            if let Some(value) = &row.sent_value {
                if q.len() as u64 >= capacity {
                    break;
                }
                row.completion = "immediate_buffered_send".into();
                q.push_back(value.clone());
            } else if let Some(value) = q.pop_front() {
                row.completion = "immediate_buffered_receive".into();
                row.queued_value_source = Some(value);
            } else {
                break;
            }
        }
        let [next] = graph.forward.successors[cursor].as_slice() else {
            break;
        };
        if graph.forward.predecessors[*next].len() != 1 {
            break;
        }
        cursor = *next;
    }
}
