//! Closed, per-call, lossless evidence tables; never a cross-invocation cache.

use super::{Dependency, MAX_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

const IDENTITY: &str = "rstyle.helper.json-transport/1";
const MAX_TREE_DEPTH: usize = 256;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    identity: String,
    root: usize,
    nodes: Vec<Value>,
}

pub(crate) fn encode(dependency: &Dependency) -> Result<Value, String> {
    let observation =
        crate::diagnostics::Stage::new("helper.transport_encode", &dependency.frontend.name, None)
            .enter();
    let value = serde_json::to_value(dependency).map_err(|error| error.to_string())?;
    check_bytes(&value)?;
    if crate::extraction_profile::ExtractionProfile::from_facts(&dependency.frontend)?
        != crate::extraction_profile::ExtractionProfile::DataV3
    {
        return observation.finish(Ok(value));
    }

    let mut table = Table::default();
    let root = table.pack(value, 0)?;
    let value = serde_json::to_value(Envelope {
        identity: IDENTITY.into(),
        root,
        nodes: table.nodes,
    })
    .map_err(|error| error.to_string())?;
    check_bytes(&value)?;
    observation.finish(Ok(value))
}

pub(crate) fn decode(value: &Value) -> Result<Dependency, String> {
    check_bytes(value)?;
    let restored = if value.get("identity").is_some() {
        let envelope: Envelope =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        restore(envelope)?
    } else {
        value.clone()
    };
    check_bytes(&restored)?;
    let dependency: Dependency =
        serde_json::from_value(restored).map_err(|error| error.to_string())?;
    if value.get("identity").is_some()
        && crate::extraction_profile::ExtractionProfile::from_facts(&dependency.frontend)?
            != crate::extraction_profile::ExtractionProfile::DataV3
    {
        return Err("Structural helper transport requires Data-v3".into());
    }

    Ok(dependency)
}

#[derive(Default)]
struct Table {
    nodes: Vec<Value>,
    indices: BTreeMap<String, usize>,
    bytes: usize,
}

impl Table {
    fn pack(&mut self, value: Value, depth: usize) -> Result<usize, String> {
        check_depth(depth)?;
        let node = self.node(value, depth)?;

        self.intern(node)
    }

    fn node(&mut self, value: Value, depth: usize) -> Result<Value, String> {
        match value {
            Value::String(text) => self.string(text, depth),
            Value::Object(object) => self.object(object, depth),
            Value::Array(array) => self.array(array, depth),
            value => Ok(serde_json::json!(["literal", value])),
        }
    }

    fn object(
        &mut self,
        object: serde_json::Map<String, Value>,
        depth: usize,
    ) -> Result<Value, String> {
        self.sequence("object", object, |table, (key, value)| {
            table
                .pack(value, depth + 1)
                .map(|id| serde_json::json!([key, id]))
        })
    }

    fn array(&mut self, array: Vec<Value>, depth: usize) -> Result<Value, String> {
        self.sequence("array", array, |table, value| {
            table.pack(value, depth + 1).map(Value::from)
        })
    }

    fn sequence<T>(
        &mut self,
        tag: &str,
        values: impl IntoIterator<Item = T>,
        mut encode: impl FnMut(&mut Self, T) -> Result<Value, String>,
    ) -> Result<Value, String> {
        let entries = values
            .into_iter()
            .map(|value| encode(self, value))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(serde_json::json!([tag, entries]))
    }

    fn string(&mut self, text: String, depth: usize) -> Result<Value, String> {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(format) = ["compact", "pretty"]
                .into_iter()
                .find(|format| render(&value, format).is_ok_and(|rendered| rendered == text))
            {
                return Ok(serde_json::json!([
                    "json",
                    format,
                    self.pack(value, depth + 1)?
                ]));
            }
            return self.lexical(&text, depth);
        }

        Ok(serde_json::json!(["literal", text]))
    }

    fn lexical(&mut self, text: &str, depth: usize) -> Result<Value, String> {
        self.sequence(
            "text",
            super::transport_lexical::fragments(text),
            |table, fragment| {
                let node = if let Some(decoded) = super::transport_lexical::quoted(fragment) {
                    serde_json::json!(["quoted", table.pack(Value::String(decoded), depth + 1)?])
                } else {
                    serde_json::json!(["literal", fragment])
                };

                table.intern(node).map(Value::from)
            },
        )
    }

    fn intern(&mut self, node: Value) -> Result<usize, String> {
        let key = render(&node, "compact")?;
        if let Some(index) = self.indices.get(&key) {
            return Ok(*index);
        }
        self.bytes = self
            .bytes
            .checked_add(key.len())
            .ok_or("Helper transport byte arithmetic failed")?;
        if self.bytes > MAX_BYTES {
            return Err("Helper transport exceeds explicit 8 MiB evidence budget".into());
        }

        let index = self.nodes.len();
        self.indices.insert(key, index);
        self.nodes.push(node);
        Ok(index)
    }
}

fn restore(envelope: Envelope) -> Result<Value, String> {
    if envelope.identity != IDENTITY || envelope.root.checked_add(1) != Some(envelope.nodes.len()) {
        return Err("Helper transport identity or root disagrees".into());
    }
    let mut reader = Reader {
        nodes: envelope.nodes,
        used: BTreeSet::new(),
        remaining: MAX_BYTES,
    };
    let value = reader.unpack(envelope.root, 0)?;
    if reader.used.len() != reader.nodes.len() {
        return Err("Unreferenced helper transport evidence".into());
    }

    Ok(value)
}

struct Reader {
    nodes: Vec<Value>,
    used: BTreeSet<usize>,
    remaining: usize,
}

impl Reader {
    fn unpack(&mut self, index: usize, depth: usize) -> Result<Value, String> {
        check_depth(depth)?;
        let fields = self
            .nodes
            .get(index)
            .and_then(Value::as_array)
            .ok_or("Missing or malformed helper transport node")?
            .clone();
        self.used.insert(index);
        self.charge(1)?;

        self.value(&fields, index, depth)
    }

    fn value(&mut self, fields: &[Value], index: usize, depth: usize) -> Result<Value, String> {
        match fields.first().and_then(Value::as_str) {
            Some("literal")
                if fields.len() == 2 && !fields[1].is_object() && !fields[1].is_array() =>
            {
                self.charge(fields[1].as_str().map_or(1, str::len))?;
                Ok(fields[1].clone())
            }
            Some("json" | "quoted") => self.encoded_string(fields, index, depth),
            Some("text" | "array" | "object") if fields.len() == 2 => {
                self.collection(fields, index, depth)
            }
            _ => Err("Unsupported or malformed helper transport tag".into()),
        }
    }

    fn encoded_string(
        &mut self,
        fields: &[Value],
        index: usize,
        depth: usize,
    ) -> Result<Value, String> {
        let text = match fields[0].as_str() {
            Some("json") if fields.len() == 3 => {
                let format = fields[1]
                    .as_str()
                    .ok_or("Malformed helper transport JSON format")?;
                let value = self.child(&fields[2], index, depth)?;
                render(&value, format)?
            }
            Some("quoted") if fields.len() == 2 => {
                let text = super::transport_lexical::string(self.child(&fields[1], index, depth)?)?;
                render(&Value::String(text), "compact")?
            }
            _ => return Err("Malformed helper transport string tag".into()),
        };
        self.charge(text.len())?;

        Ok(Value::String(text))
    }

    fn collection(
        &mut self,
        fields: &[Value],
        index: usize,
        depth: usize,
    ) -> Result<Value, String> {
        match fields[0].as_str() {
            Some("array") => {
                let ids = fields[1]
                    .as_array()
                    .ok_or("Malformed helper transport array")?;
                ids.iter()
                    .map(|id| self.child(id, index, depth))
                    .collect::<Result<_, _>>()
                    .map(Value::Array)
            }
            Some("text") => self.text(&fields[1], index, depth),
            Some("object") => self.object(&fields[1], index, depth),
            _ => Err("Malformed helper transport collection tag".into()),
        }
    }

    fn child(&mut self, value: &Value, parent: usize, depth: usize) -> Result<Value, String> {
        let index = value
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < parent)
            .ok_or("Cyclic, forward or invalid helper transport index")?;
        self.unpack(index, depth + 1)
    }

    fn object(&mut self, value: &Value, index: usize, depth: usize) -> Result<Value, String> {
        let entries = value
            .as_array()
            .ok_or("Malformed helper transport object")?;
        let mut object = serde_json::Map::new();
        for entry in entries {
            let fields = entry
                .as_array()
                .filter(|fields| fields.len() == 2)
                .ok_or("Malformed helper transport object entry")?;
            let key = fields[0]
                .as_str()
                .ok_or("Malformed helper transport object key")?;
            self.charge(key.len())?;
            let value = self.child(&fields[1], index, depth)?;
            if object.insert(key.into(), value).is_some() {
                return Err("Duplicate helper transport object key".into());
            }
        }

        Ok(Value::Object(object))
    }

    fn text(&mut self, value: &Value, index: usize, depth: usize) -> Result<Value, String> {
        let ids = value
            .as_array()
            .ok_or("Malformed lexical helper transport fragments")?;
        let mut writer = BoundedWriter(Vec::new());
        for id in ids {
            let text = super::transport_lexical::string(self.child(id, index, depth)?)?;
            writer
                .write_all(text.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        let text = String::from_utf8(writer.0).map_err(|error| error.to_string())?;
        self.charge(text.len())?;

        Ok(Value::String(text))
    }

    fn charge(&mut self, bytes: usize) -> Result<(), String> {
        crate::evidence_budget::charge(
            &mut self.remaining,
            bytes,
            "Helper restoration exceeds explicit 8 MiB work budget",
        )
    }
}

fn check_depth(depth: usize) -> Result<(), String> {
    if depth > MAX_TREE_DEPTH {
        return Err("Helper transport exceeds explicit tree-depth budget 256".into());
    }

    Ok(())
}

struct BoundedWriter(Vec<u8>);

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other(
                "Helper transport exceeds explicit 8 MiB byte budget",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn render(value: &Value, format: &str) -> Result<String, String> {
    let mut writer = BoundedWriter(Vec::new());
    match format {
        "compact" => serde_json::to_writer(&mut writer, value),
        "pretty" => serde_json::to_writer_pretty(&mut writer, value),
        _ => return Err("Unsupported helper JSON-string format".into()),
    }
    .map_err(|error| error.to_string())?;

    String::from_utf8(writer.0).map_err(|error| error.to_string())
}

fn check_bytes(value: &Value) -> Result<(), String> {
    render(value, "compact").map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(value: Value) -> Envelope {
        let mut table = Table::default();
        let root = table.pack(value, 0).unwrap();
        Envelope {
            identity: IDENTITY.into(),
            root,
            nodes: table.nodes,
        }
    }

    #[test]
    fn restores_exact_literals_json_formats_and_tag_like_source_objects() {
        let original = serde_json::json!({
            "compact": "{\"a\":[1,true]}", "pretty": "{\n  \"a\": 1\n}",
            "literal": "{ \"a\": 1 }\r\n", "unicode": "\u{feff}source\r\n",
            "nested": "{\"text\":\"{\\\"a\\\":1}\"}",
            "ordered": "{\"z\":\"a\\\"b\\\\c\", \"a\":\"\\u0061\"}\r\n",
            "tag_like": {"identity": IDENTITY, "nodes": ["literal", 1]},
        });
        assert_eq!(restore(envelope(original.clone())).unwrap(), original);
    }

    #[test]
    fn rejects_malformed_unused_cyclic_duplicate_and_over_budget_evidence() {
        for nodes in [
            vec![serde_json::json!(["other", 1])],
            vec![serde_json::json!(["array", [0]])],
            vec![serde_json::json!(["literal", {}])],
            vec![
                serde_json::json!(["literal", 1]),
                serde_json::json!(["literal", 2]),
            ],
            vec![
                serde_json::json!(["literal", 1]),
                serde_json::json!(["json", "other", 0]),
            ],
            vec![
                serde_json::json!(["literal", 1]),
                serde_json::json!(["object", [["a", 0], ["a", 0]]]),
            ],
        ] {
            let root = nodes.len() - 1;
            assert!(
                restore(Envelope {
                    identity: IDENTITY.into(),
                    root,
                    nodes
                })
                .is_err()
            );
        }
        assert!(check_depth(257).is_err());
        assert!(check_bytes(&Value::String("x".repeat(MAX_BYTES))).is_err());
    }

    #[test]
    fn independent_lexical_table_restores_exact_ordered_document_bytes() {
        let nodes = vec![
            serde_json::json!(["literal", "{\"z\":1, "]),
            serde_json::json!(["literal", "a"]),
            serde_json::json!(["quoted", 1]),
            serde_json::json!(["literal", ":\"\\u0061\"}\r\n"]),
            serde_json::json!(["text", [0, 2, 3]]),
        ];
        assert_eq!(
            restore(Envelope {
                identity: IDENTITY.into(),
                root: 4,
                nodes
            })
            .unwrap(),
            Value::String("{\"z\":1, \"a\":\"\\u0061\"}\r\n".into())
        );
    }

    #[test]
    fn shares_only_identical_transport_nodes_and_bounds_expansion() {
        let value = serde_json::json!([[1, 2, 3], [1, 2, 3], [3, 2, 1]]);
        let table = envelope(value.clone());
        let ids = table.nodes[table.root][1].as_array().unwrap();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert_eq!(restore(table).unwrap(), value);
        let mut nodes = vec![serde_json::json!(["literal", "x".repeat(1024)])];
        for index in 0..15 {
            nodes.push(serde_json::json!(["array", [index, index]]));
        }
        assert!(
            restore(Envelope {
                identity: IDENTITY.into(),
                root: 15,
                nodes
            })
            .is_err()
        );
    }
}
