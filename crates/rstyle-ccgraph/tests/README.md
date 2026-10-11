# Imported-graph test conventions

These tests materialize the compact stage fixtures in
`vendor-compat/fixtures/ccgraph-matching.json` as typed inputs. The compact
encoding is hand-worked stage input, not PDG-schema documents (see that
file's `authoring` note).

Flag conventions for `projection_cases[].input.nodes`:

- `administrative` marks positively identified administrative boundary
  bookkeeping (profile §2, rule 5). Absent means not administrative.
- `reachable` marks entry reachability for rule 4. Absent means reachable.
- `synthetic` is informational here. Retention follows from the node
  category under the profile rules (e.g. formal inputs are kept per
  rule 2); in production it is a real PDG schema field.
- `read_symbols` on reads feeds the recomputed reference count.

Neither `administrative` nor `reachable` is a PDG schema field. In
production, reachability comes from BFS over the paired CFG entry
(`rstyle-pdg::reachable_vertices` plus the operation-map correspondence),
and administrative-ness is proven from provenance plus the absence of
binding semantics (`rstyle-pdg` validation).
