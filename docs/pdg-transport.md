# Shared PDG model

`PdgDocument` directly implements the current shared PDG 0.1.0 schema. Its
ordinary serde representation is the only import/export format: one root
`profile`, no record-level `policy`, and one `extraction_evidence.conformance`
object containing `status` and `reasons`.

Use `serde_json::to_vec(&pdg)` and `serde_json::from_slice::<PdgDocument>(bytes)`.
Paired documents, borrowed pair views, shared unit bundles, byte manifests and
serialized verification all use this same model. There is no legacy model,
projection, profile inheritance, migration adapter or compatibility promise.
Manifests use the current `checksum` field and `xxh3-64` byte checksums directly;
the old `sha256` field alias and manifest SHA fallback are removed.

The source/build and graph verifier still checks the decoded graph. Schema
compatibility alone does not establish extraction conformance. Counts retain
their known/value and unavailable/reason alternatives. Nodes, directed parallel
edges, binding/definition references, operand positions, branch outcomes,
evidence, provenance and source spans are represented directly. Namespaced
annotations and extensions retain their opaque dependency proof data.

The native model targets `vendor-compat/pdg.schema.json`, verified against
compat main revision `3e191191a1cf67f9cbcacb270d9ab28d16410284`, SHA256
`d2ce26b478c2c01620d50f08ac7aea32036fa105424fb73f6b9adbf68c4f3291`.
Normative vendored snapshots are unchanged. The six paired Go/Rust cases in
compat retain their independent graph expectations.
