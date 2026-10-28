use super::{Arm, Assessment, Candidate, MethodOrigin, Signature};
use crate::ByteSpan;
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, Item, Type, spanned::Spanned, visit::Visit};

pub(super) struct SourceFailure {
    pub reason: String,
    pub budget_exhausted: bool,
}

pub(super) struct Module {
    pub functions: Vec<syn::ItemFn>,
    reason: Option<&'static str>,
    owners: BTreeSet<String>,
    enums: BTreeMap<String, Enum>,
    traits: BTreeMap<String, Trait>,
    methods: Vec<Method>,
    foreign_impls: bool,
}

#[derive(Default)]
struct ItemFacts {
    functions: Vec<syn::ItemFn>,
    methods: Vec<Method>,
    namespace: NamespaceUpdate,
}

#[derive(Default)]
struct NamespaceUpdate {
    declaration: Option<Declaration>,
    reason: Option<&'static str>,
    foreign_impls: bool,
}

enum Declaration {
    Struct(String),
    Enum(String, Enum),
    Trait(String, Trait),
}

impl Declaration {
    fn conflict_reason(&self) -> &'static str {
        match self {
            Self::Struct(_) => "Struct owner has ambiguous local identity",
            Self::Enum(_, _) => "Enum owner has ambiguous local identity",
            Self::Trait(_, _) => "Trait has ambiguous local identity",
        }
    }
}

struct MethodFacts {
    function: syn::ItemFn,
    method: Method,
}

struct Enum {
    header: ByteSpan,
    variants: Vec<(String, String)>,
    supported: bool,
}
struct Trait {
    header: ByteSpan,
    members: BTreeSet<String>,
    supported: bool,
}
struct Method {
    owner: String,
    name: String,
    header: ByteSpan,
    origin: Option<String>,
    shape: MethodShape,
}
struct MethodShape {
    shared: bool,
    ordinary: usize,
    scalar: bool,
    supported: bool,
}

pub(super) fn parse(source: &str) -> Result<Vec<Module>, SourceFailure> {
    parse_budget(source)?;
    let file = syn::parse_file(source).map_err(|error| SourceFailure {
        reason: format!("Original file syntax unavailable: {error}"),
        budget_exhausted: false,
    })?;

    let unavailable = NamespaceClosure::inspect(&file)?;
    let reason = (!file.attrs.is_empty())
        .then_some("Original file attributes exceed the dispatch source contract");
    let mut modules = IndexBuilder::build(source, file.items, reason)?;

    if unavailable || modules.iter().any(|module| module.foreign_impls) {
        for module in &mut modules {
            module.reason = Some(
                "Unobserved declarations, expansion or nonlocal impl owners prevent closed dispatch resolution",
            );
        }
    }

    Ok(modules)
}

#[derive(Default)]
struct NamespaceClosure {
    unavailable: bool,
    items: usize,
}

impl NamespaceClosure {
    fn inspect(file: &syn::File) -> Result<bool, SourceFailure> {
        let mut closure = Self::default();
        closure.visit_file(file);
        if closure.items > 4096 {
            return Err(budget("Original declaration scan budget exhausted"));
        }
        Ok(closure.unavailable)
    }
}

impl<'ast> Visit<'ast> for NamespaceClosure {
    fn visit_item(&mut self, item: &'ast Item) {
        self.items += 1;
        syn::visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        self.items += 1;
        syn::visit::visit_impl_item(self, item);
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        self.items += 1;
        syn::visit::visit_trait_item(self, item);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.unavailable |= block
            .stmts
            .iter()
            .any(|statement| matches!(statement, syn::Stmt::Item(_)));
        syn::visit::visit_block(self, block);
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.unavailable = true;
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.unavailable = true;
    }
}

fn parse_budget(source: &str) -> Result<(), SourceFailure> {
    if source.len() > 256 * 1024 {
        return Err(budget(
            "Original source byte budget exhausted before parsing",
        ));
    }
    let mut depth = 0_usize;
    for byte in source.bytes() {
        match byte {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth > 256 {
            return Err(budget(
                "Original source delimiter budget exhausted before parsing",
            ));
        }
    }
    Ok(())
}

fn budget(reason: &str) -> SourceFailure {
    SourceFailure {
        reason: reason.into(),
        budget_exhausted: true,
    }
}

struct IndexBuilder<'a> {
    source: &'a str,
    remaining: usize,
    modules: Vec<Module>,
}

#[derive(Clone, Copy)]
struct ModuleScope {
    reason: Option<&'static str>,
    depth: usize,
}

impl Module {
    fn new(reason: Option<&'static str>) -> Self {
        Self {
            functions: vec![],
            reason,
            owners: BTreeSet::new(),
            enums: BTreeMap::new(),
            traits: BTreeMap::new(),
            methods: vec![],
            foreign_impls: false,
        }
    }

    fn apply(&mut self, facts: ItemFacts) {
        if !facts.functions.is_empty() {
            self.functions.extend(facts.functions);
        }
        if !facts.methods.is_empty() {
            self.methods.extend(facts.methods);
        }
        self.apply_namespace(facts.namespace);
    }

    fn apply_namespace(&mut self, update: NamespaceUpdate) {
        let duplicate = update
            .declaration
            .and_then(|declaration| self.insert_declaration(declaration));
        let reason = duplicate.or(update.reason);
        if let Some(reason) = reason {
            self.reason = Some(reason);
        }
        if update.foreign_impls {
            self.foreign_impls = true;
        }
    }

    fn insert_declaration(&mut self, declaration: Declaration) -> Option<&'static str> {
        let duplicate = match &declaration {
            Declaration::Struct(name) => self.owners.contains(name),
            Declaration::Enum(name, _) => self.enums.contains_key(name),
            Declaration::Trait(name, _) => self.traits.contains_key(name),
        };
        let reason = declaration.conflict_reason();

        match declaration {
            Declaration::Struct(name) => {
                self.owners.insert(name);
            }
            Declaration::Enum(name, declaration) => {
                self.enums.insert(name, declaration);
            }
            Declaration::Trait(name, declaration) => {
                self.traits.insert(name, declaration);
            }
        }
        duplicate.then_some(reason)
    }

    fn close_namespace(&mut self) {
        self.foreign_impls |= self.methods.iter().any(|method| {
            !self.owners.contains(&method.owner) && !self.enums.contains_key(&method.owner)
        });
        if self.owners.contains("u32")
            || self.enums.contains_key("u32")
            || self.traits.contains_key("u32")
        {
            self.reason = Some("Local declaration shadows primitive u32");
        }
    }
}

impl<'a> IndexBuilder<'a> {
    fn build(
        source: &'a str,
        items: Vec<Item>,
        reason: Option<&'static str>,
    ) -> Result<Vec<Module>, SourceFailure> {
        let mut index = Self {
            source,
            remaining: 4096,
            modules: vec![],
        };
        index.index_modules(items, ModuleScope { reason, depth: 0 })?;
        Ok(index.modules)
    }

    fn index_modules(&mut self, items: Vec<Item>, scope: ModuleScope) -> Result<(), SourceFailure> {
        if scope.depth > 32 || items.len() > self.remaining {
            return Err(budget("Original module/item index budget exhausted"));
        }
        self.remaining -= items.len();

        let mut module = Module::new(scope.reason);
        for item in items {
            module.apply(self.index_item(item, scope)?);
        }

        module.close_namespace();
        self.modules.push(module);
        Ok(())
    }

    fn index_item(&mut self, item: Item, scope: ModuleScope) -> Result<ItemFacts, SourceFailure> {
        Ok(match item {
            Item::Mod(item) => self.index_module(item, scope)?,
            Item::Fn(item) => ItemFacts {
                functions: vec![item],
                ..ItemFacts::default()
            },
            Item::Struct(item) => struct_facts(item),
            Item::Enum(item) => enum_facts(item, self.source),
            Item::Trait(item) => trait_facts(item, self.source, &mut self.remaining)?,
            Item::Impl(item) => self.index_impl(item)?,
            Item::Const(_) | Item::Static(_) => ItemFacts::default(),
            _ => ItemFacts::unavailable(
                "Module imports, aliases or opaque items prevent exact local dispatch resolution",
            ),
        })
    }

    fn index_module(
        &mut self,
        item: syn::ItemMod,
        scope: ModuleScope,
    ) -> Result<ItemFacts, SourceFailure> {
        let reason = if !item.attrs.is_empty() || item.unsafety.is_some() {
            Some("Module attributes or unsafe module ownership are unsupported")
        } else {
            scope.reason
        };
        if let Some((_, items)) = item.content {
            self.index_modules(
                items,
                ModuleScope {
                    reason,
                    depth: scope.depth + 1,
                },
            )?;
            Ok(ItemFacts::default())
        } else {
            Ok(ItemFacts {
                namespace: NamespaceUpdate {
                    foreign_impls: true,
                    ..NamespaceUpdate::default()
                },
                ..ItemFacts::default()
            })
        }
    }
}

impl ItemFacts {
    fn unavailable(reason: &'static str) -> Self {
        Self {
            namespace: NamespaceUpdate {
                reason: Some(reason),
                ..NamespaceUpdate::default()
            },
            ..Self::default()
        }
    }

    fn declaration(declaration: Declaration, reason: Option<&'static str>) -> Self {
        Self {
            namespace: NamespaceUpdate {
                declaration: Some(declaration),
                reason,
                ..NamespaceUpdate::default()
            },
            ..Self::default()
        }
    }
}

fn struct_facts(item: syn::ItemStruct) -> ItemFacts {
    let unsupported = !item.attrs.is_empty()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.fields.iter().any(|field| !field.attrs.is_empty());
    ItemFacts::declaration(
        Declaration::Struct(item.ident.to_string()),
        unsupported.then_some("Struct owner has attributes, generics or ambiguous identity"),
    )
}

fn enum_facts(item: syn::ItemEnum, source: &str) -> ItemFacts {
    let variants: Vec<_> = item
        .variants
        .iter()
        .filter_map(|variant| {
            let syn::Fields::Unnamed(fields) = &variant.fields else {
                return None;
            };
            if fields.unnamed.len() != 1
                || !variant.attrs.is_empty()
                || variant.discriminant.is_some()
            {
                return None;
            }
            let field = &fields.unnamed[0];
            field
                .attrs
                .is_empty()
                .then(|| plain_type(&field.ty))
                .flatten()
                .map(|owner| (variant.ident.to_string(), owner))
        })
        .collect();
    let supported = item.attrs.is_empty()
        && item.generics.params.is_empty()
        && item.generics.where_clause.is_none()
        && item.variants.len() == 2
        && variants.len() == 2
        && variants[0].1 != variants[1].1;
    let header = ByteSpan {
        start: span(source, item.span()).start,
        end: span(source, item.brace_token.span.open()).start,
    };
    ItemFacts::declaration(
        Declaration::Enum(
            item.ident.to_string(),
            Enum {
                header,
                variants,
                supported,
            },
        ),
        None,
    )
}

fn trait_facts(
    item: syn::ItemTrait,
    source: &str,
    remaining: &mut usize,
) -> Result<ItemFacts, SourceFailure> {
    if item.items.len() > *remaining {
        return Err(budget("Original trait member index budget exhausted"));
    }
    *remaining -= item.items.len();

    let members = item
        .items
        .iter()
        .filter_map(|member| match member {
            syn::TraitItem::Fn(item) => Some(item.sig.ident.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let supported = item.attrs.is_empty() && item.unsafety.is_none() && item.auto_token.is_none()
        && item.generics.params.is_empty() && item.generics.where_clause.is_none() && item.supertraits.is_empty()
        && item.items.iter().all(|member| matches!(member, syn::TraitItem::Fn(item) if item.attrs.is_empty() && method_shape(&item.sig).supported));
    let header = ByteSpan {
        start: span(source, item.span()).start,
        end: span(source, item.brace_token.span.open()).start,
    };

    Ok(ItemFacts::declaration(
        Declaration::Trait(
            item.ident.to_string(),
            Trait {
                header,
                members,
                supported,
            },
        ),
        None,
    ))
}

struct ImplOwner {
    name: Option<String>,
    origin: Option<String>,
}

impl ImplOwner {
    fn from_item(item: &syn::ItemImpl) -> Self {
        Self {
            name: plain_type(&item.self_ty),
            origin: item
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| plain_path(path)),
        }
    }

    fn nonlocal(&self, item: &syn::ItemImpl) -> bool {
        self.name.is_none() || (item.trait_.is_some() && self.origin.is_none())
    }

    fn unsupported(&self, item: &syn::ItemImpl) -> bool {
        !item.attrs.is_empty()
            || item.unsafety.is_some()
            || !item.generics.params.is_empty()
            || item.generics.where_clause.is_some()
            || self.nonlocal(item)
            || item
                .trait_
                .as_ref()
                .is_some_and(|(negative, _, _)| negative.is_some())
    }
}

impl IndexBuilder<'_> {
    fn index_impl(&mut self, item: syn::ItemImpl) -> Result<ItemFacts, SourceFailure> {
        if item.items.len() > self.remaining {
            return Err(budget("Original impl member index budget exhausted"));
        }
        self.remaining -= item.items.len();

        let owner = ImplOwner::from_item(&item);
        let reason = item
            .items
            .iter()
            .filter_map(impl_member_reason)
            .next_back()
            .or_else(|| {
                owner.unsupported(&item).then_some(
                    "Impl owner, trait origin or configuration exceeds the local source contract",
                )
            });
        let namespace = NamespaceUpdate {
            reason,
            foreign_impls: owner.nonlocal(&item),
            ..NamespaceUpdate::default()
        };
        let (functions, methods) = item
            .items
            .into_iter()
            .filter_map(|member| {
                let syn::ImplItem::Fn(function) = member else {
                    return None;
                };
                let facts = method_facts(self.source, function, &owner);
                Some((facts.function, facts.method))
            })
            .unzip();

        Ok(ItemFacts {
            functions,
            methods,
            namespace,
        })
    }
}

fn impl_member_reason(member: &syn::ImplItem) -> Option<&'static str> {
    match member {
        syn::ImplItem::Fn(function) if !function.attrs.is_empty() => {
            Some("Method attributes prevent exact selected origin")
        }
        syn::ImplItem::Fn(_) => None,
        _ => Some("Non-method impl items exceed the local dispatch contract"),
    }
}

fn method_facts(source: &str, function: syn::ImplItemFn, owner: &ImplOwner) -> MethodFacts {
    let method = Method {
        owner: owner.name.clone().unwrap_or_default(),
        name: function.sig.ident.to_string(),
        header: signature_header(source, &function.vis, &function.sig),
        origin: owner.origin.clone(),
        shape: method_shape(&function.sig),
    };

    MethodFacts {
        method,
        function: syn::ItemFn {
            attrs: function.attrs,
            vis: function.vis,
            sig: function.sig,
            block: Box::new(function.block),
        },
    }
}

fn method_shape(signature: &syn::Signature) -> MethodShape {
    let shared = signature.receiver().is_some_and(|receiver| {
        receiver.reference.is_some()
            && receiver.mutability.is_none()
            && receiver.colon_token.is_none()
    });
    MethodShape {
        shared,
        ordinary: signature
            .inputs
            .len()
            .saturating_sub(usize::from(signature.receiver().is_some())),
        scalar: returns_u32(signature),
        supported: bounded_signature(signature),
    }
}

fn bounded_signature(signature: &syn::Signature) -> bool {
    signature.unsafety.is_none()
        && signature.asyncness.is_none()
        && signature.constness.is_none()
        && signature.abi.is_none()
        && signature.generics.params.is_empty()
        && signature.generics.where_clause.is_none()
        && signature.variadic.is_none()
}

fn returns_u32(signature: &syn::Signature) -> bool {
    matches!(&signature.output, syn::ReturnType::Type(_, ty) if plain_type(ty).as_deref() == Some("u32"))
}

pub(super) fn function_header(source: &str, function: &syn::ItemFn) -> ByteSpan {
    signature_header(source, &function.vis, &function.sig)
}

pub(super) fn function_range(source: &str, function: &syn::ItemFn) -> ByteSpan {
    ByteSpan {
        start: function_header(source, function).start,
        end: span(source, function.block.span()).end,
    }
}

fn signature_header(
    source: &str,
    visibility: &syn::Visibility,
    signature: &syn::Signature,
) -> ByteSpan {
    let signature_span = span(source, signature.span());
    ByteSpan {
        start: if matches!(visibility, syn::Visibility::Inherited) {
            signature_span.start
        } else {
            span(source, visibility.span()).start
        },
        end: signature_span.end,
    }
}

fn span(source: &str, span: proc_macro2::Span) -> ByteSpan {
    crate::source_evidence::original_span(source, span)
}

fn plain_type(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.qself
        .is_none()
        .then(|| plain_path(&path.path))
        .flatten()
}

fn plain_path(path: &syn::Path) -> Option<String> {
    (path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].arguments.is_empty())
    .then(|| path.segments[0].ident.to_string())
}

enum Decision {
    Negative(&'static str),
    Unsupported(&'static str),
}
type DispatchResult<T> = std::result::Result<T, Decision>;

pub(super) fn evaluate(module: &Module, function: &syn::ItemFn, source: &str) -> Assessment {
    let expression = match function.block.stmts.as_slice() {
        [syn::Stmt::Expr(Expr::Match(expression), None)] => expression,
        statements
            if statements
                .iter()
                .any(|statement| matches!(statement, syn::Stmt::Expr(Expr::Match(_), _))) =>
        {
            return Assessment::unsupported("Dispatch match has additional surrounding statements");
        }
        _ => return Assessment::negative("No top-level enum dispatch witness"),
    };
    if let Some(reason) = module.reason {
        return Assessment::unsupported(reason);
    }
    match candidate(module, function, expression, source) {
        Ok(candidate) => Assessment {
            availability: super::super::Availability::Known(Some(candidate)),
            reason: None,
            budget_exhausted: false,
        },
        Err(Decision::Negative(reason)) => Assessment::negative(reason),
        Err(Decision::Unsupported(reason)) => Assessment::unsupported(reason),
    }
}

fn candidate(
    module: &Module,
    function: &syn::ItemFn,
    expression: &syn::ExprMatch,
    source: &str,
) -> DispatchResult<Candidate> {
    check_dispatch_shape(function, expression)?;
    let name = enum_parameter(function, &expression.expr)?;
    let declaration = module.enums.get(&name).ok_or(Decision::Unsupported(
        "Match receiver is not a unique same-module enum",
    ))?;
    check_enum_payloads(module, declaration)?;

    let dispatch = DispatchSource {
        module,
        source,
        name: &name,
        declaration,
    };
    let arms = dispatch.inspect_arms(expression)?;
    if matches!((&arms[0].origin, &arms[1].origin), (MethodOrigin::LocalTrait { trait_name: left, member: first, .. }, MethodOrigin::LocalTrait { trait_name: right, member: second, .. }) if left == right && first == second)
    {
        return Err(Decision::Negative(
            "Selected methods already implement the same exact local trait member",
        ));
    }

    Ok(Candidate {
        enum_name: name,
        caller_header: function_header(source, function),
        enum_header: declaration.header,
        signature: Signature {
            shared_self: true,
            ordinary_parameters: 0,
            return_type: "u32",
        },
        arms,
        advisory: "Consider a shared local capability for these payload methods; review effects, lifetimes and dispatch before editing.",
    })
}

fn check_dispatch_shape(function: &syn::ItemFn, expression: &syn::ExprMatch) -> DispatchResult<()> {
    if !function.attrs.is_empty()
        || !bounded_signature(&function.sig)
        || !returns_u32(&function.sig)
        || function.sig.receiver().is_some()
    {
        return Err(Decision::Unsupported(
            "Caller signature exceeds plain free/shared enum dispatch",
        ));
    }
    if !expression.attrs.is_empty()
        || expression.arms.len() != 2
        || expression
            .arms
            .iter()
            .any(|arm| arm.guard.is_some() || !arm.attrs.is_empty())
    {
        return Err(Decision::Unsupported(
            "Dispatch requires exactly two exhaustive unguarded original arms",
        ));
    }
    Ok(())
}

fn check_enum_payloads(module: &Module, declaration: &Enum) -> DispatchResult<()> {
    if !declaration.supported
        || declaration
            .variants
            .iter()
            .any(|(_, owner)| !module.owners.contains(owner))
    {
        return Err(Decision::Unsupported(
            "Enum requires two distinct original local struct payload variants",
        ));
    }
    Ok(())
}

fn enum_parameter(function: &syn::ItemFn, expression: &Expr) -> DispatchResult<String> {
    let Expr::Path(path) = expression else {
        return Err(Decision::Unsupported(
            "Match scrutinee is not a plain formal read",
        ));
    };
    let name = path
        .qself
        .is_none()
        .then(|| plain_path(&path.path))
        .flatten()
        .ok_or(Decision::Unsupported(
            "Match scrutinee path is not local/plain",
        ))?;
    for argument in &function.sig.inputs {
        let syn::FnArg::Typed(argument) = argument else {
            continue;
        };
        let syn::Pat::Ident(pattern) = &*argument.pat else {
            continue;
        };
        if pattern.ident != name {
            continue;
        }
        let Type::Reference(reference) = &*argument.ty else {
            break;
        };
        if reference.mutability.is_none()
            && reference.lifetime.is_none()
            && pattern.by_ref.is_none()
            && pattern.subpat.is_none()
        {
            return plain_type(&reference.elem).ok_or(Decision::Unsupported(
                "Enum parameter type requires an unsupported path",
            ));
        }
    }
    Err(Decision::Unsupported(
        "Match formal requires a plain shared enum reference",
    ))
}

struct DispatchSource<'a> {
    module: &'a Module,
    source: &'a str,
    name: &'a str,
    declaration: &'a Enum,
}

impl DispatchSource<'_> {
    fn inspect_arms(&self, expression: &syn::ExprMatch) -> DispatchResult<Vec<Arm>> {
        let mut seen = BTreeSet::new();
        let mut arms = vec![];
        for arm in &expression.arms {
            let (variant, _) = arm_pattern(&arm.pat, self.name)?;
            if !seen.insert(variant) {
                return Err(Decision::Unsupported(
                    "Duplicate variant arm is not exhaustive correspondence",
                ));
            }
            arms.push(self.inspect_arm(arm, expression)?);
        }
        Ok(arms)
    }

    fn inspect_arm(&self, arm: &syn::Arm, expression: &syn::ExprMatch) -> DispatchResult<Arm> {
        let (variant, binder) = arm_pattern(&arm.pat, self.name)?;
        let owner = self
            .declaration
            .variants
            .iter()
            .find(|(name, _)| name == &variant)
            .map(|(_, owner)| owner)
            .ok_or(Decision::Unsupported(
                "Arm variant is outside the declared enum",
            ))?;

        let Expr::MethodCall(call) = &*arm.body else {
            return Err(if same_free_calls(expression, self.name) {
                Decision::Negative(
                    "Both arms call the same free callee; no payload method dispatch",
                )
            } else {
                Decision::Unsupported("Opaque or additional arm work cannot be stripped")
            });
        };

        let method = self.selected_call(call, &binder, owner)?;
        Ok(Arm {
            variant,
            arm_span: span(self.source, arm.span()),
            receiver_type: owner.clone(),
            method: method.name.clone(),
            callee_header: method.header,
            origin: method_origin(self.module, method)?,
        })
    }

    fn selected_call(
        &self,
        call: &syn::ExprMethodCall,
        binder: &str,
        owner: &str,
    ) -> DispatchResult<&Method> {
        if !call.attrs.is_empty() || call.turbofish.is_some() {
            return Err(Decision::Unsupported(
                "Method call attributes or substitutions are unsupported",
            ));
        }
        if !matches!(&*call.receiver, Expr::Path(path) if path.qself.is_none() && path.path.is_ident(binder))
        {
            return Err(Decision::Negative(
                "Selected call receiver is not its arm payload binder",
            ));
        }

        let method = selected_method(self.module, owner, &call.method.to_string())?;
        check_call_shape(method, call)?;
        Ok(method)
    }
}

fn check_call_shape(method: &Method, call: &syn::ExprMethodCall) -> DispatchResult<()> {
    if !method.shape.supported {
        return Err(Decision::Unsupported(
            "Selected method signature is generic/unsafe or otherwise unavailable",
        ));
    }
    if !method.shape.shared
        || method.shape.ordinary != 0
        || !method.shape.scalar
        || !call.args.is_empty()
    {
        return Err(Decision::Negative(
            "Selected method signatures do not share zero-argument shared-self/u32 shape",
        ));
    }
    Ok(())
}

fn arm_pattern(pattern: &syn::Pat, enum_name: &str) -> DispatchResult<(String, String)> {
    let syn::Pat::TupleStruct(pattern) = pattern else {
        return Err(Decision::Unsupported(
            "Arm requires exact enum tuple-variant binding",
        ));
    };
    let path = &pattern.path;
    if pattern.qself.is_some()
        || path.leading_colon.is_some()
        || path.segments.len() != 2
        || path.segments[0].ident != enum_name
        || path.segments.iter().any(|part| !part.arguments.is_empty())
        || pattern.elems.len() != 1
    {
        return Err(Decision::Unsupported(
            "Arm enum path or payload roster is unsupported",
        ));
    }

    let syn::Pat::Ident(binder) = &pattern.elems[0] else {
        return Err(Decision::Unsupported(
            "Arm payload must be a single plain binder",
        ));
    };
    if binder.by_ref.is_some() || binder.mutability.is_some() || binder.subpat.is_some() {
        return Err(Decision::Unsupported(
            "Arm payload binding mode is unsupported",
        ));
    }
    Ok((path.segments[1].ident.to_string(), binder.ident.to_string()))
}

fn same_free_calls(expression: &syn::ExprMatch, enum_name: &str) -> bool {
    let names: Option<Vec<_>> = expression.arms.iter().map(|arm| {
        let (_, binder) = arm_pattern(&arm.pat, enum_name).ok()?;
        let Expr::Call(call) = &*arm.body else { return None; };
        if !call.attrs.is_empty() || call.args.len() != 1 { return None; }
        let Expr::Field(field) = &call.args[0] else { return None; };
        if !field.attrs.is_empty() || !matches!(field.member, syn::Member::Named(_)) { return None; }
        if !matches!(&*field.base, Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() && path.path.is_ident(&binder)) { return None; }
        match &*call.func {
            Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() => plain_path(&path.path),
            _ => None,
        }
    }).collect();
    names.is_some_and(|names| names.len() == 2 && names[0] == names[1])
}

fn selected_method<'a>(module: &'a Module, owner: &str, name: &str) -> DispatchResult<&'a Method> {
    let candidates: Vec<_> = module
        .methods
        .iter()
        .filter(|method| method.owner == owner && method.name == name)
        .collect();
    let inherent: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|method| method.origin.is_none())
        .collect();
    match inherent.as_slice() {
        [method] if !method.shape.shared && candidates.len() > 1 => Err(Decision::Unsupported(
            "Competing inherent and trait receivers require resolved method-selection evidence",
        )),
        [method] => Ok(method),
        [] if candidates.len() == 1 => Ok(candidates[0]),
        _ => Err(Decision::Unsupported(
            "Selected method origin is missing or ambiguous",
        )),
    }
}

fn method_origin(module: &Module, method: &Method) -> DispatchResult<MethodOrigin> {
    let Some(name) = &method.origin else {
        return Ok(MethodOrigin::Inherent);
    };
    let declaration = module.traits.get(name).ok_or(Decision::Unsupported(
        "Selected trait origin is not an original local declaration",
    ))?;
    if !declaration.supported || !declaration.members.contains(&method.name) {
        return Err(Decision::Unsupported(
            "Selected trait member origin lacks an exact local declaration",
        ));
    }
    Ok(MethodOrigin::LocalTrait {
        trait_name: name.clone(),
        member: method.name.clone(),
        trait_header: declaration.header,
    })
}
