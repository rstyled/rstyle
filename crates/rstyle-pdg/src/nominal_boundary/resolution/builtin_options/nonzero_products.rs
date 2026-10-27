//! Exact caller-scope type bindings after the independent output cleanup proof.
use super::*;

impl Resolution {
    pub(in crate::nominal_boundary::resolution) fn verify_nonzero_binding(
        &self,
        facts: &HirFnFacts,
        namespace: &[String],
        scalar: &str,
    ) -> Result<(), String> {
        let modules = self
            .environment
            .modules
            .iter()
            .filter(|module| module.namespace == namespace)
            .collect::<Vec<_>>();

        let [module] = modules.as_slice() else {
            return Err("Nonzero source requires one exact caller module".into());
        };

        let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
        Scope {
            environment: &self.environment,
            module,
            blocks: Vec::new(),
            configuration: &configuration,
            facts,
        }
        .nonzero_binding(scalar)
    }
}

impl Scope<'_, '_> {
    fn nonzero_binding(&self, scalar: &str) -> Result<(), String> {
        let binding = self
            .lookup("NonZero")?
            .ok_or("Nonzero source lacks its explicit core type import")?;

        if binding != ["core", "num", "NonZero"] && binding != ["std", "num", "NonZero"] {
            return Err("Nonzero source type import resolves to another definition".into());
        }

        if self
            .lookup(&binding[0])?
            .is_some_and(|root| root != [binding[0].clone()])
            || self.lookup(scalar)?.is_some()
        {
            return Err("Nonzero source shadows its library root or concrete primitive".into());
        }

        let function: syn::ItemFn =
            syn::parse_str(&self.facts.source.bytes).map_err(|error| error.to_string())?;

        let syn::ReturnType::Type(_, output) = &function.sig.output else {
            return Err("Nonzero source lacks its original Option result spelling".into());
        };

        let syn::Type::Path(path) = output.as_ref() else {
            return Err("Nonzero source result must retain its original Option path".into());
        };

        self.product_option(path)?;
        Ok(())
    }
}

impl Resolution {
    pub(crate) fn verify_nonzero_member_scope(
        &self,
        member: &HirFnFacts,
        product: &crate::concrete_types::Nominal,
    ) -> Result<(), String> {
        let mut namespace: Vec<_> = product
            .definition_path
            .split("::")
            .map(str::to_owned)
            .collect();
        namespace
            .pop()
            .ok_or("Missing callback product namespace")?;

        let file = member
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing callback source file")?;

        let modules: Vec<_> = self
            .environment
            .modules
            .iter()
            .filter(|module| {
                module.namespace == namespace && module.path == std::path::Path::new(file)
            })
            .collect();

        let [module] = modules.as_slice() else {
            return Err("Callback member lacks one exact product source module".into());
        };

        let configuration = crate::conditional_source::Configuration::new(&member.build.cfgs);

        let scope = Scope {
            environment: &self.environment,
            module,
            blocks: Vec::new(),
            configuration: &configuration,
            facts: member,
        };
        scope.nonzero_member_names(product)
    }
}

impl Scope<'_, '_> {
    fn nonzero_member_names(&self, product: &crate::concrete_types::Nominal) -> Result<(), String> {
        for (name, path) in [
            ("Some", &["option", "Option", "Some"][..]),
            ("Option", &["option", "Option"][..]),
            ("FnOnce", &["ops", "FnOnce"][..]),
        ] {
            if let Some(binding) = self.lookup(name)? {
                let expected: Vec<_> = std::iter::once("core")
                    .chain(path.iter().copied())
                    .map(str::to_owned)
                    .collect();
                let alternate: Vec<_> = std::iter::once("std")
                    .chain(path.iter().copied())
                    .map(str::to_owned)
                    .collect();
                if binding != expected && binding != alternate {
                    return Err(
                        "Callback member shadows its builtin Option, variant or callable trait"
                            .into(),
                    );
                }
            }
        }

        let expected: Vec<_> = product
            .definition_path
            .split("::")
            .map(str::to_owned)
            .collect();

        if !self.environment.index.declarations.contains_key(&expected) {
            return Err(
                "Callback constructor does not resolve to its exact checked product".into(),
            );
        }
        for root in ["core", "std"] {
            if self.lookup(root)?.is_some_and(|binding| binding != [root]) {
                return Err("Callback member shadows its pinned sysroot root".into());
            }
        }
        Ok(())
    }
}
