# Caller-visible scalar mutation effects

Request the optional consumer separately from direct mutation budgets:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --locked --mutation-effects --format json --report-detail compact
```

`mutation_effects` (`rstyle.mutation-effects/2`) describes original direct writes
and calls through at most two original same-selected-unit helper edges:
`caller → wrapper → leaf`. It does not require strict clone admission. The
vocabulary covers ordinary nongeneric free functions with owned `u32` or direct
`&mut u32` inputs, `u32` or unit results, and primitive scalar bodies. A leaf is
call-free; a wrapper requires every original call to resolve to a complete leaf.
Third-edge chains, external functions, function pointers, recursion, opaque
arguments, aliases, reference redirection, closures, unsafe operations and
nontrivial owned values remain incomplete. Checked direct witnesses in an
assessed function survive an unknown remainder; an incomplete wrapper cannot
supply a complete summary.

The separate compiler `consumer_formal_types` observation records direct mutable
builtin `u32` formal types. Source spelling and rendered compiler type names do
not establish primitive referents. This observation supplies no graph admission,
alias, access or rewrite permission. Original source ownership, native binding
identity, signatures, call arguments and selected-unit context still have to agree.

Each function's evidence reports `caller_visible_mutation`:

| Value | Meaning |
| --- | --- |
| `none_proven` | Complete bounded analysis found no caller-visible scalar writes. |
| `writes_formal` | Complete bounded analysis found writes through mutable formals. |
| `witnessed_writes_formal_with_unknown_remainder` | Checked write witnesses survive incomplete evidence elsewhere. |
| `unknown` | Incomplete evidence with no checked caller-visible write witness. |

These values make no claim about purity, panic freedom, termination, general
effects, dynamic write frequency, density or safe extraction. `none_proven` is
limited to this mutation vocabulary. An unused unsupported formal still prevents
a complete result: dropping a value may execute behavior absent from explicit
assignments.

Direct assignment-site counts stay with their original owner. Callee-owned local
rebinding does not become a caller write. Every summarized original call has its
own event, while repeated calls reuse completed leaf and wrapper proofs in the
owning analysis. Repeated inner calls retain separate paths even when they reach
the same original assignment. Argument mapping projects the original leaf formal
into the wrapper first, then the wrapper formal into the caller. Root sets
deduplicate only identical owner-scoped bindings; witness occurrences stay distinct.

Function `unit` and call `summary_index` select the report's summary row with the
same `unit` and `summary_index`. Summary `function_index` is the original owning
unit's function ordinal. `summary_site_index` indexes that summary's `sites`
witness roster. Each witness
retains `function_index` and `site_index` of its original direct assignment,
`call_path` of original helper call occurrences, and `mapped_root` projected into
the summary owner. The flattened `span`, `root`, `operation` and `caller_visible`
fields belong to the original write. Write spans are relative to that original
function's source. A direct witness has an empty path; a nested witness has one
wrapper call reference with its owner, occurrence index, original span and native
identity. The outer call event supplies the remaining edge. No nested leaf site
is presented as a wrapper direct assignment. These are report-local handles,
never stable review identities or executable serialized proof authority.

`--max-analysis-functions` bounds inspected owners including required callees.
Additional bounded call, summary and witness work can also exhaust the analysis.
Retained summary sites, path entries and expanded call mappings consume the same
existing witness allowance; shared proof does not make repeated expansion free.
Summary queries distinguish remaining depth, so a depth-limited result cannot
supply a deeper proof. Complete call-free leaf proofs are shared across depths.
Omitted prerequisites remain incomplete. Requested incomplete effect scope exits
two; the consumer adds no mutation-budget findings. Direct `--mutation-rules`
configuration and metrics remain separate. Default runs do not request effects.

An explicit `--replay-run PATH --mutation-effects` reevaluates retained original
facts without source files or compiler tools. It recomputes summaries inside each
immutable owning unit; it never imports a serialized clean-effect verdict or
shares summaries across analyses or selected units.
