//! Move typed records through policy transport without a document JSON round trip.
use crate::{Extensions, ProfileId, R0_POLICY_ID, R0_POLICY_VERSION, producer::Pair};
use serde_json::Value;

pub(super) fn apply(pair: Pair, policy: &ProfileId) -> Result<Pair, String> {
    super::shape::check(&pair)?;
    let transport = Transport { policy };
    Ok(Pair {
        pdg: transport.pdg(pair.pdg),
        cfg: crate::CfgDocument {
            producer: crate::CfgProducer {
                configuration: transport.map(pair.cfg.producer.configuration),
                ..pair.cfg.producer
            },
            extensions: transport.map(pair.cfg.extensions),
            ..pair.cfg
        },
    })
}

struct Transport<'a> {
    policy: &'a ProfileId,
}

impl Transport<'_> {
    fn pdg(&self, doc: crate::PdgDocument) -> crate::PdgDocument {
        crate::PdgDocument {
            profile: self.profile(doc.profile),
            producer: crate::Producer {
                configuration: self.map(doc.producer.configuration),
                ..doc.producer
            },
            function: self.interface(doc.function),
            symbols: doc
                .symbols
                .into_iter()
                .map(|symbol| crate::Symbol {
                    extensions: self.map(symbol.extensions),
                    ..symbol
                })
                .collect(),
            memory_locations: doc
                .memory_locations
                .into_iter()
                .map(|location| self.location(location))
                .collect(),
            nodes: doc.nodes.into_iter().map(|node| self.node(node)).collect(),
            edges: doc.edges.into_iter().map(|edge| self.edge(edge)).collect(),
            extraction_evidence: self.evidence(doc.extraction_evidence),
            extensions: self.map(doc.extensions),
            ..doc
        }
    }

    fn interface(&self, function: crate::FunctionInfo) -> crate::FunctionInfo {
        crate::FunctionInfo {
            interface_completeness: self.described(function.interface_completeness),
            inputs: function
                .inputs
                .into_iter()
                .map(|parameter| self.parameter(parameter))
                .collect(),
            outputs: function
                .outputs
                .into_iter()
                .map(|parameter| self.parameter(parameter))
                .collect(),
            ..function
        }
    }

    fn parameter(&self, parameter: crate::Parameter) -> crate::Parameter {
        crate::Parameter {
            extensions: self.map(parameter.extensions),
            ..parameter
        }
    }

    fn location(&self, location: crate::MemoryLocation) -> crate::MemoryLocation {
        crate::MemoryLocation {
            description: self.text(location.description),
            evidence: self.described(location.evidence),
            extensions: self.map(location.extensions),
            ..location
        }
    }

    fn node(&self, node: crate::PdgNode) -> crate::PdgNode {
        crate::PdgNode {
            evidence: self.described(node.evidence),
            provenance: self.described(node.provenance),
            attributes: crate::NodeAttributes {
                description: node.attributes.description.map(|text| self.text(text)),
                extensions: self.map(node.attributes.extensions),
                ..node.attributes
            },
            annotations: self.map(node.annotations),
            ..node
        }
    }

    fn edge(&self, edge: crate::PdgEdge) -> crate::PdgEdge {
        crate::PdgEdge {
            evidence: self.described(edge.evidence),
            provenance: self.described(edge.provenance),
            annotations: self.map(edge.annotations),
            ..edge
        }
    }

    fn evidence(&self, evidence: crate::ExtractionEvidence) -> crate::ExtractionEvidence {
        crate::ExtractionEvidence {
            capabilities: evidence
                .capabilities
                .into_iter()
                .map(|(key, value)| (key, self.described(value)))
                .collect(),
            ..evidence
        }
    }

    fn described<T: Described>(&self, mut value: T) -> T {
        let description = value.description();
        *description = self.text(std::mem::take(description));
        value
    }

    fn profile(&self, policy: ProfileId) -> ProfileId {
        if policy.id == R0_POLICY_ID && policy.version == R0_POLICY_VERSION {
            self.policy.clone()
        } else {
            policy
        }
    }

    fn text(&self, text: String) -> String {
        if text.contains("R0") || text.contains("compat.rust.hir-r0/0.1.0") {
            text.replace("R0", "local scalar").replace(
                "compat.rust.hir-r0/0.1.0",
                &format!("{}/{}", self.policy.id, self.policy.version),
            )
        } else {
            text
        }
    }

    fn map(&self, map: Extensions) -> Extensions {
        let identity = Self::identity(map.get("id"), map.get("version"));
        map.into_iter()
            .map(|(key, value)| {
                let value = self.field(&key, value, identity);
                (key, value)
            })
            .collect()
    }

    fn value(&self, value: Value) -> Value {
        match value {
            Value::Object(object) => {
                let identity = Self::identity(object.get("id"), object.get("version"));
                Value::Object(
                    object
                        .into_iter()
                        .map(|(key, value)| {
                            let value = self.field(&key, value, identity);
                            (key, value)
                        })
                        .collect(),
                )
            }
            Value::Array(values) => {
                Value::Array(values.into_iter().map(|value| self.value(value)).collect())
            }
            value => value,
        }
    }

    fn identity(id: Option<&Value>, version: Option<&Value>) -> bool {
        id.and_then(Value::as_str) == Some(R0_POLICY_ID)
            && version.and_then(Value::as_str) == Some(R0_POLICY_VERSION)
    }

    fn field(&self, key: &str, value: Value, identity: bool) -> Value {
        match key {
            "id" if identity => Value::String(self.policy.id.clone()),
            "version" if identity => Value::String(self.policy.version.clone()),
            "dependency" => value,
            "description" | "normal_flow_scope" => match value {
                Value::String(text) => Value::String(self.text(text)),
                value => self.value(value),
            },
            _ => self.value(value),
        }
    }
}

/// Only extraction descriptions change when selecting a local profile.
trait Described {
    fn description(&mut self) -> &mut String;
}

impl Described for crate::Capability {
    fn description(&mut self) -> &mut String {
        &mut self.description
    }
}

impl Described for crate::Provenance {
    fn description(&mut self) -> &mut String {
        &mut self.description
    }
}
