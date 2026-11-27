//! Sequential operations on private bounded channels. A blocked operation, a goroutine,
//! or an escape outside the modeled function ends the walk.
use super::{Node, NodeKind, Scalar, TypeKind, Walk};
use crate::cpg::model::Type;
use std::collections::VecDeque;

pub(super) struct Channel {
    capacity: usize,
    closed: bool,
    queue: VecDeque<Scalar>,
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn channel_element(&self, ty: &Type) -> Option<&'a Type> {
        if ty.kind != TypeKind::Opaque {
            return None;
        }
        let fact = self
            .facts
            .tables
            .extension(&self.profile.channel_types)?
            .get(&ty.id)?;
        if !matches!(
            fact.get("direction")?.as_str()?,
            "both" | "send" | "receive"
        ) {
            return None;
        }
        self.facts.type_of(fact.get("element")?.as_str()?)
    }

    pub(super) fn channel_builtin(
        &mut self,
        node: &'a Node,
        op: &str,
        depth: usize,
    ) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        if op == self.profile.make
            && arg.kind == NodeKind::TypeAnnotation
            && (2..=3).contains(&node.children.len())
        {
            let capacity = if node.children.len() == 3 {
                self.index(self.child(node, 2)?, depth)?
            } else {
                0
            };
            if capacity > super::storage::MAX_ITEMS
                || self.channels.len() >= super::storage::MAX_ITEMS
            {
                return None;
            }
            let slot = self.channels.len();
            self.channels.push(Channel {
                capacity,
                closed: false,
                queue: VecDeque::new(),
            });
            return Some(Scalar::Channel(Some(slot)));
        }
        if node.children.len() != 2 {
            return None;
        }
        let Scalar::Channel(slot) = self.value(arg, depth)? else {
            return None;
        };
        if op == self.profile.byte_length || op == self.profile.capacity {
            let len = match slot {
                None => 0,
                Some(s) if op == self.profile.capacity => self.channels.get(s)?.capacity,
                Some(s) => self.channels.get(s)?.queue.len(),
            };
            return Some(Scalar::Int(len as i128));
        }
        if op != self.profile.close || self.write_at != Some(node.id.as_str()) {
            return None;
        }
        let c = self.channels.get_mut(slot?)?;
        if c.closed {
            return None;
        }
        c.closed = true;
        Some(Scalar::Nil)
    }

    pub(super) fn send(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        let source = self.child(node, 0)?;
        let Scalar::Channel(Some(slot)) = self.value(source, depth)? else {
            return None;
        };
        let value = self.value(self.child(node, 1)?, depth)?;
        let channel = self.channels.get_mut(slot)?;
        if channel.closed || channel.capacity <= channel.queue.len() {
            return None;
        }
        channel.queue.push_back(value);
        Some(())
    }

    pub(super) fn receive(&mut self, node: &'a Node, depth: usize) -> Option<(Scalar, bool)> {
        let ch = self.child(node, 0)?;
        let Scalar::Channel(Some(slot)) = self.value(ch, depth)? else {
            return None;
        };
        let channel = self.channels.get_mut(slot)?;
        if let Some(v) = channel.queue.pop_front() {
            return Some((v, true));
        }
        if !channel.closed {
            return None;
        }
        Some((self.zero(self.channel_element(self.facts.ty(ch)?)?)?, false))
    }
}
