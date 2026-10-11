# Imported-graph test conventions

These tests materialize the compact stage fixtures in
`vendor-compat/fixtures/ccgraph-matching.json` as typed inputs. The compact
encoding is hand-worked stage input, not PDG-schema documents (see that
file's `authoring` note). Sections are stage-isolated on purpose: a WL
fixture passing says nothing about size/numerical/string gates, and
fixture passage is not producer conformance or detection quality.

## Shared encodings

- `graph_nodes` maps navigation aliases to one of the five initial labels.
  Aliases are arbitrary: renaming them must not change scores.
- `graph_edges` lists `[source,target,family]` tuples; each tuple is one
  distinct occurrence. No endpoint deduplication, ever.
- `fractions` are `[numerator,denominator]` exact rationals. Threshold and
  score comparisons in these tests use them directly, never floats.
- `cosine` objects hold `{numerator,sqrt_denominator}`, meaning
  `numerator/sqrt(sqrt_denominator)`; self-kernels enter only as their
  product, matching the exact squared comparison.
- `histogram_labels` are per-round equivalence-class tokens shared across
  the compared graphs, not implementation hashes or stable IDs.

## Section by section

- `projection_cases`: extracted-stage inputs with `administrative`,
  `reachable`, `synthetic`, and `read_symbols` node flags (see below).
  `expected` pins retained/removed nodes, retained edges with multiplicity,
  the nine numeric features, and a `forbidden` list of wrong
  implementations that must not produce those numbers.
- `interface_cases`: `(role, type, variadic)` tuples plus a name, pinned to
  exact `IntroPara`/`ReturnPara`/`Name` serializations.
- `jaro_winkler_cases`: string pairs pinned to exact fractions, including
  empty, transposition, orientation, Unicode, and below-0.7-boost behavior.
  Every value is also checked symmetrically.
- `diameter_cases`: directed max-finite-distance expectations, including
  the undirected-path-is-not-directed-path, self-edge, cycle (stage-only,
  never R0), and parallel-edge cases.
- `depth_cases`: pair-depth selection from node counts and diameters,
  including the equal-size tie and the zero-depth pair.
- `wl_cases`: full refinement expectations per pair — depth, weights,
  per-round overlap/cross/self counts, both scores. Includes direction,
  family, and multiplicity sensitivities plus the documented zero-depth
  self-loop limitation (WL stage only; other gates may differ).
- `histogram_score_cases`: scoring arithmetic straight from hand-written
  histograms, without asserting any source graph realizes them. Includes
  the exact-0.9 overlap boundary and the demonstration that raw
  cross-product over min node count is unbounded.
- `boundary_cases`: inclusive gate boundaries as exact checks — size ratio
  (including zero ratio not admitting empty projections), rational scores,
  the three-field string mean, and squared-comparison cosine.
- `future_harness_requirements`: not executable here. Each entry names a
  coverage obligation for a future producer/detector harness; several are
  already covered at the imported-graph stage by `pipeline_tests`.

## Projection-case flag conventions

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

## Reading the projection expectations

`expected.features` is the nine-coordinate numeric vector in fixed order
(`NumOfCtrlEdges, NumOfExcEdges, NumOfDataEdges, NumOfDeclNodes,
NumOfAssignNodes, NumOfCtrlNodes, NumOfStateNodes, NumOfOtherNodes,
NumOfReference`), recomputed from the retained slice: edge occurrences keep
multiplicity and family, the five node counts sum to the projected size,
and the reference count covers distinct symbols read by retained reads
only (unreachable reads do not count).

`expected.forbidden` lists wrong implementations that must not produce
those numbers: removing synthetic formal inputs, splicing boundary-node
neighbors, counting unreachable reads, and deduplicating distinct edge
families that share endpoints.
