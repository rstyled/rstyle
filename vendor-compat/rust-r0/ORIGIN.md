# Vendored R0 extraction contracts (test inputs only)

Pinned source revision: `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`
(reference: https://github.com/rstyled/compat/tree/04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f)

Snapshotted paths (read from that revision, not from a moving branch):

- `rust-extraction-profile.md` (from `schemas/rust-extraction-profile.md`)
- `rust-capabilities.md` (from `schemas/rust-capabilities.md`)
- `fixtures/rust-r0-extraction.json` (from `schemas/fixtures/rust-r0-extraction.json`)
- `pdg.schema.json` (from `schemas/pdg.schema.json`)
- `cfg.schema.json` (from `schemas/cfg.schema.json`)
- `ccgraph-decisions.md` (from `schemas/ccgraph-decisions.md`)
- `ccgraph-profile.md` (from `schemas/ccgraph-profile.md`)
- `ccgraph-profile.json` (from `schemas/ccgraph-profile.json`)
- `README.md` (from `schemas/README.md`)

Normative targets: extraction profile `compat.rust.hir-r0 / 0.1.0`,
PDG `0.1.0`, CFG `0.2.0`.

This directory is a **snapshot for self-contained tests**, not a second
authoritative specification. The authoritative contracts live in the compat
repository at the pinned revision above. Do not hand-edit these files to make
tests pass; if the contract changes, re-vendor from a new pinned revision and
record it here.

The pinned PDG schema requires record policies on edges and known reference
counts and an array of profile-specific conformance assessments. The later
workspace transport uses a root profile and a single assessment. The producer's
checked pinned-wire bridge emits the authoritative format, validates policy
agreement, then converts to existing transport for strict admission. See
`R0_COVERAGE_LEDGER.md`; these snapshots have not been modified.
