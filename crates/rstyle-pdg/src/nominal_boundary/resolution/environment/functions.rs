//! Lookup handles into a checked, immutable source inventory; never admission proof.
use super::*;

#[derive(Clone, Copy)]
enum Handle {
    Function(usize),
    Method {
        implementation: usize,
        member: usize,
    },
}

#[derive(Default)]
pub(super) struct Index(BTreeMap<(usize, u32, u32), Vec<Handle>>);

impl Index {
    pub(super) fn new(modules: &[census::Module], census: &census::Census) -> Result<Self, String> {
        let mut index = Self::default();
        for (module_id, module) in modules.iter().enumerate() {
            let bytes = file_bytes(census, &module.path)?;
            index.module(module_id, bytes, &module.items);
        }
        Ok(index)
    }

    fn module(&mut self, module: usize, bytes: &str, items: &[syn::Item]) {
        let mut builder = ModuleIndex {
            index: self,
            module,
            bytes,
        };
        for (position, item) in items.iter().enumerate() {
            builder.item(position, item);
        }
    }

    fn handles(&self, module: usize, span: ByteSpan) -> impl Iterator<Item = Handle> + '_ {
        self.0
            .get(&(module, span.start, span.end))
            .into_iter()
            .flatten()
            .copied()
    }
}

struct ModuleIndex<'a> {
    index: &'a mut Index,
    module: usize,
    bytes: &'a str,
}

impl ModuleIndex<'_> {
    fn item(&mut self, position: usize, item: &syn::Item) {
        match item {
            syn::Item::Fn(item) => self.insert(
                FunctionSource {
                    vis: &item.vis,
                    sig: &item.sig,
                    body: &item.block,
                },
                Handle::Function(position),
            ),
            syn::Item::Impl(item) => self.methods(position, item),
            _ => {}
        }
    }

    fn methods(&mut self, position: usize, item: &syn::ItemImpl) {
        for (member, method) in item.items.iter().enumerate() {
            if let syn::ImplItem::Fn(method) = method {
                self.insert(
                    FunctionSource {
                        vis: &method.vis,
                        sig: &method.sig,
                        body: &method.block,
                    },
                    Handle::Method {
                        implementation: position,
                        member,
                    },
                );
            }
        }
    }

    fn insert(&mut self, function: FunctionSource<'_>, handle: Handle) {
        let span = function.range(self.bytes);
        self.index
            .0
            .entry((self.module, span.start, span.end))
            .or_default()
            .push(handle);
    }
}

impl Environment {
    pub(in crate::nominal_boundary::resolution) fn owner_candidates(
        &self,
        module: usize,
        span: ByteSpan,
    ) -> impl Iterator<Item = (&syn::Item, Option<&syn::ImplItemFn>)> {
        self.functions
            .handles(module, span)
            .map(move |handle| match handle {
                Handle::Function(position) => (&self.modules[module].items[position], None),
                Handle::Method {
                    implementation,
                    member,
                } => {
                    let item = &self.modules[module].items[implementation];
                    let syn::Item::Impl(owner) = item else {
                        unreachable!("Method handle retains its original implementation")
                    };
                    let syn::ImplItem::Fn(method) = &owner.items[member] else {
                        unreachable!("Method handle retains its original function member")
                    };
                    (item, Some(method))
                }
            })
    }
}

#[cfg(test)]
mod tests;
