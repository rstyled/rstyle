# Vendored compat snapshot (test inputs only)

Pinned source revision: `283f1fba27f1`
(reference: https://github.com/rstyled/compat/tree/283f1fba27f1)

Snapshotted paths (read from that revision, not from a moving branch):

- `ccgraph-profile.json` (from `schemas/ccgraph-profile.json`)
- `pdg.schema.json` (from `schemas/pdg.schema.json`)
- `cfg.schema.json` (from `schemas/cfg.schema.json`)
- `cfg.example.json` (from `schemas/cfg.example.json`)
- `pdg.example.json` (from `schemas/pdg.example.json`)
- `fixtures/ccgraph-matching.json` (from `schemas/fixtures/ccgraph-matching.json`)

This directory is a **snapshot for self-contained tests**, not a second
authoritative specification. The authoritative contracts live in the compat
repository at the pinned revision above. Do not hand-edit these files to make
tests pass; if the contract changes, re-vendor from a new pinned revision and
record it here.
