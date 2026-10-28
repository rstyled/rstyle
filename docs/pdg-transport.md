# Shared PDG transport

The producer exports the current shared PDG 0.1.0 grammar through
`rstyle_pdg::transport`: one root `profile`, no record-level `policy` fields,
and one `extraction_evidence.conformance` object containing `status` and
`reasons`. `producer::Pair`, `PairView`, shared unit bundles, `Pair::serialize`,
and `verify_serialized` all use this boundary. The existing strict source/build
and graph verifier still runs after decoding; schema compatibility does not
establish extraction conformance.

For a standalone PDG, serialize `transport::Borrowed(&pdg)` or
`transport::Document(pdg)`, and deserialize `transport::Document`. The direct
serde grammar of the internal `PdgDocument` remains historical so internal
verification and pinned historical fixtures retain their policy inventories.
It is not the current shared wire encoding.

Export rejects conflicting internal record policies, a conformance assessment
for a different profile, and zero or multiple conformance assessments. It never
chooses a claim or silently discards a conflicting profile. Import rejects the
obsolete record fields and array form, then inherits the root profile into
internal verification metadata. Counts retain their known/value or
unavailable/reason alternatives. Old wire documents require an explicit
migration; the current reader does not silently accept them.

Nodes, directed parallel edges, binding/definition references, operand positions,
branch outcomes, evidence statuses/reasons, provenance and source spans are
preserved. Namespaced annotations and extensions are opaque and unchanged,
including dependency proof inventories. The encoder streams borrowed typed
records without cloning the document or constructing a JSON value tree.

Transport and driver emission tests use `vendor-compat/pdg.schema.json`, rather
than the historical `vendor-compat/rust-r0` schema. On 2026-10-28, the top-level
schema was verified byte-for-byte against compat main revision
`3e191191a1cf67f9cbcacb270d9ab28d16410284`, SHA256
`d2ce26b478c2c01620d50f08ac7aea32036fa105424fb73f6b9adbf68c4f3291`.
The normative snapshots and their historical provenance are unchanged.

Regression checks cover the current independent example, paired and standalone
exports, byte-manifest identities, rejection of obsolete fields and conflicting
claims, count alternatives, graph attributes/multiplicity, and opaque proofs.
The six paired Go/Rust cases remain independently specified in compat; this
transport change does not repair source spans or upgrade evidence claims.
