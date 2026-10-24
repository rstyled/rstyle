//! Source-only transport declarations, retained before nominal permission selection.
use super::*;
use crate::{
    declaration_origins::{GeneratedOrigin, Origins},
    generated_sources::{capture::UnitCapture, checked::CheckedMember, declarations},
    result_maps::source::{self, CheckedTransportSource},
};
use selected::no_drop::CheckedNoDrop;

pub(super) struct TransportSources {
    source: CheckedTransportSource,
    generated: Vec<declarations::CheckedGeneratedDeclaration>,
    certificates: Vec<(TypeTree, CheckedNoDrop)>,
    census: Arc<census::Census>,
}

pub(super) struct Input<'a> {
    pub facts: &'a HirFnFacts,
    pub resolution: &'a Resolution,
    pub origins: &'a Origins,
    pub member: CheckedMember,
    pub capture: &'a UnitCapture,
}

pub(super) fn capture(input: Input<'_>) -> Result<Option<TransportSources>, String> {
    let Some(source) = source::capture(input.facts, input.resolution, input.origins, input.member)?
    else {
        return Ok(None);
    };
    let request = source.request();
    let mut certificates = selected::no_drop::transport_copy_sources(
        input.facts,
        input.resolution,
        input.origins,
        request,
    )?;
    let error = request.cleanup_requests()[1];
    let declaration = generated_declaration(input.facts, input.resolution, &source, input.capture)?;

    let generated = selected::no_drop::generated_tuple_errors::verify(
        input.facts,
        input.resolution,
        request,
        &declaration,
    )?;
    let payload = request.cleanup_requests()[0];
    let original = selected::no_drop::verify(input.facts, payload, &input.resolution.environment)?;
    certificates.extend([(payload.clone(), original), (error.clone(), generated)]);
    let census = input.resolution.environment.census_owner();
    Ok(Some(TransportSources {
        source,
        generated: vec![declaration],
        certificates,
        census,
    }))
}

fn generated_declaration(
    facts: &HirFnFacts,
    resolution: &Resolution,
    source: &CheckedTransportSource,
    capture: &UnitCapture,
) -> Result<declarations::CheckedGeneratedDeclaration, String> {
    let error = source.request().cleanup_requests()[1];
    let origins = facts.build.frontend_options.data::<Origins>()?;
    let origin = origin(&origins, error)?;
    super::verify_declaration_fields(resolution, facts, source.member().namespace(), error)?;
    validate_generated(facts, &origins, &resolution.environment, error)?;

    declarations::verify(declarations::Input {
        facts,
        census: resolution.environment.census(),
        capture,
        member: source.member(),
        tree: error,
        origin,
    })
}

impl TransportSources {
    pub(super) fn source(&self) -> &CheckedTransportSource {
        &self.source
    }
    pub(super) fn declaration(&self, tree: &TypeTree) -> Option<&CheckedNoDrop> {
        self.certificates
            .iter()
            .find_map(|(checked, receipt)| (checked == tree).then_some(receipt))
    }
    pub(super) fn generated(
        &self,
        tree: &TypeTree,
    ) -> Option<&declarations::CheckedGeneratedDeclaration> {
        self.generated
            .iter()
            .find(|declaration| declaration.tree() == tree)
    }
    fn source_type(&self, ty: &syn::Type) -> Result<&str, String> {
        let source = self.source.member().expansion().source_span(ty.span());
        let file = self
            .census
            .files
            .iter()
            .find(|file| file.path == source.path && file.digest == source.digest)
            .ok_or("Transport signature lacks exact original source file")?;
        crate::source_evidence::byte_range(&file.bytes, source.span)
    }
}

pub(super) fn validate_generated(
    facts: &HirFnFacts,
    origins: &Origins,
    environment: &Environment,
    tree: &TypeTree,
) -> Result<bool, String> {
    let Some((nominal, definition, _, _)) = tree.generated_parts() else {
        return Ok(false);
    };

    environment.check_context(facts)?;
    let observed = origin(origins, tree)?;
    let census = environment.census();
    let package = &census.context.package;
    if &observed.nominal != nominal
        || &observed.definition != definition
        || nominal.source_identity.as_deref() != Some(&package.package_id)
        || nominal.package_name != package.name
        || nominal.package_version != package.version
        || nominal.manifest_digest != package.manifest_digest
        || !census.files.iter().any(|file| {
            file.path == observed.origin.raw.path
                && file.digest == observed.origin.raw.digest
                && file.bytes == observed.file_bytes
        })
    {
        return Err(
            "Generated declaration differs from exact original source and selected package".into(),
        );
    }
    Ok(true)
}

fn origin<'a>(origins: &'a Origins, tree: &TypeTree) -> Result<&'a GeneratedOrigin, String> {
    let nominal = tree
        .nominal_identity()
        .ok_or("Generated source requires a nominal type")?;

    let mut matching = origins
        .generated
        .iter()
        .filter(|origin| &origin.nominal == nominal);
    let origin = matching
        .next()
        .ok_or("Missing exact generated declaration origin")?;
    if matching.next().is_some() {
        return Err("Duplicate generated declaration origin".into());
    }
    Ok(origin)
}

impl Resolution {
    pub(super) fn transport_named_type(
        &self,
        namespace: &[String],
        path: &syn::TypePath,
    ) -> Option<TypeTree> {
        let sources = self.transport_sources.as_ref()?;
        if path.qself.is_some() || path.path.leading_colon.is_some() {
            return None;
        }
        sources.generated.iter().find_map(|declaration| {
            (namespace == declaration.namespace() && path.path.is_ident(&declaration.item().ident))
                .then(|| declaration.tree().clone())
        })
    }

    pub(crate) fn transport_source(&self) -> Option<&CheckedTransportSource> {
        self.transport_sources
            .as_ref()
            .map(TransportSources::source)
    }
    pub(crate) fn transport_type_source(&self, ty: &syn::Type) -> Result<&str, String> {
        self.transport_sources
            .as_ref()
            .ok_or("Missing checked transport source signature")?
            .source_type(ty)
    }
    pub(super) fn bind_transport_function(&mut self, facts: &HirFnFacts) -> Result<bool, String> {
        let Some(source) = &self.transport_sources else {
            return Ok(false);
        };
        source.source.verify(facts)?;
        let member = source.source.member();
        signature(facts, member.function())?;
        let owner = source.source.request().copy_requests()[1];
        let nominal = owner
            .nominal_identity()
            .ok_or("Transport success wrapper lacks its owner")?;
        let syn::Type::Path(path) = member.implementation().self_ty.as_ref() else {
            return Err("Transport source implementation lacks a nominal owner".into());
        };
        let namespace = member.namespace().to_vec();
        let scoped = Scoped {
            resolution: self,
            namespace: &namespace,
        };
        let owner_path = scoped.name(path)?;
        if owner_path.join("::") != nominal.definition_path {
            return Err(
                "Transport function implementation differs from exact success wrapper".into(),
            );
        }

        selected::check_transport_namespace(facts, self, member.namespace())?;
        let receiver = owner.clone();
        self.namespace = namespace;
        self.owner_path = Some(owner_path);
        self.receiver = Some(receiver);
        Ok(true)
    }
}

fn signature(facts: &HirFnFacts, function: &syn::ImplItemFn) -> Result<(), String> {
    let signature = &function.sig;
    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != facts.signature.params.len()
        || signature
            .inputs
            .iter()
            .any(|argument| matches!(argument, syn::FnArg::Receiver(_)))
        || facts.signature.has_self
    {
        return Err("Transport function requires its exact ordinary static signature".into());
    }
    selected::function_attributes(facts, &function.attrs)
}
