# Vendored compat snapshot (test inputs only)

Pinned source revision: `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`
(reference: https://github.com/rstyled/compat/tree/04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f)

Snapshotted paths (read from that revision, not from a moving branch):

- `ccgraph-profile.json` (from `schemas/ccgraph-profile.json`)
- `pdg.schema.json` (from `schemas/pdg.schema.json`)
- `cfg.schema.json` (from `schemas/cfg.schema.json`)
- `fixtures/ccgraph-matching.json` (from `schemas/fixtures/ccgraph-matching.json`)
- `cfg.example.json` (from `schemas/cfg.example.json`)

This directory is a **snapshot for self-contained tests**, not a second
authoritative specification. The authoritative contracts live in the compat
repository at the pinned revision above. Do not hand-edit these files to make
tests pass; if the contract changes, re-vendor from a new pinned revision and
record it here.
