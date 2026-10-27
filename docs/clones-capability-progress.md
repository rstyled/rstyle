# Clones capability migration progress

Execution starts at `dd7f4fb` on 2026-10-27. The supplied execution prompt and
migration plan define the scope; this ledger records delivered slices rather
than declaring the wider migration complete. The [matrix](clones-capability-matrix.md)
and [acceptance corpus](../eval/clones-capability/README.md) freeze the first scope.

## Baseline and ownership

The starting revision passed 2,332 workspace tests (five skipped), formatting,
Clippy, and both legacy dog-food gates: zero style violations across 8,200
functions and zero reviewed pattern candidates. Logs retained for that exact
revision: `/tmp/rstyle-clean-{tests,style,patterns,clippy}.log` and corresponding
style/pattern JSON. These results precede migration implementation; checkpoint
gates will be repeated. Native Linux aarch64 only; other platform gates and exact
Go behavior comparisons remain unverified.

Pre-existing untracked work, including the supplied migration documents,
`slice_access/`, `docs/pdg-and-clones.md`, scalar-loop coverage fixtures and the
PDF, is excluded from task commits. The lead owns shared registration, report/CLI,
this ledger, and Git staging. Evidence owns `consumer_facts`; local consumers own
`analysis/source_rules`; acceptance owns `eval/clones-capability` and the matrix.
Heavy captures and integrated gates run serially.

## Delivery ledger

| ID | Owner / write set | State | Commit | Capability / scope | Evidence and remaining work |
| --- | --- | --- | --- | --- | --- |
| C01 | Acceptance / corpus and matrix | delivered (acceptance specification) | `112c085` | Independently labeled source/signature boundaries, excluded-body case, strict pair and negative abstraction; prepared guard/difference cases | JSON/count checks, fixture formatting and offline Cargo check; producer acceptance not yet claimed |
| C02 | Evidence / `consumer_facts` and lead registration | delivered (focused) | `0d97fd8` | Source/interface availability independent of strict graph admission | Eight frozen focused source ownership/signature tests passed; no new extraction permission; integrated gates pending |
| C03 | Lead / clones report and CLI acceptance | delivered (focused) | pair-output commit | Source-located accepted strict pairs | Two real-Cargo tests passed: full/compact/text locations agree; pair budget remains incomplete; integrated gates pending |
| C04 | Local + lead / source rules and opt-in CLI | implementing | pending | Length/parameters with separate availability, execution, scope and outcomes | Existing pure code-line policy reused; receiver count normalized; no graph prerequisite |
| C05 | Acceptance + lead / comparison | ready | pending | Compare same-scope legacy/new results against independent labels | Separate unavailable coverage, evaluated agreement, reviewed improvement and regression |
| C06–C08 | Local / pair consumers | ready (acceptance preparation) | pending | Bounded guard opportunities and ordered pair differences, then helper proposals | Prepared labels are requirements, not support claims |

No capability is retired. Stable review identity, broader proposals, conservative
effects/mutation and work planning remain later required migration work.

## Hashing exceptions

New consumers use borrowed evidence and report-local typed handles, without new
hashes. Existing `source_identity::Scheme::LegacySha256` and
`concrete_types::digest` support actual historical source claims and frozen
source/oracle pins; immutable owners compute them only for those explicit claims.
The `clones::digest` helper is test-only historical test-contract compatibility,
not a new finding identity. New routine source publication uses existing XXH3.
This bounded inventory does not claim every historical fixture has migrated.
