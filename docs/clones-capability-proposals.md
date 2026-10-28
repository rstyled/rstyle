# Witnessed local capability proposals

Request advisory proposals independently of strict clone matching:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --locked --capability-proposals --format json --report-detail compact
```

The initial consumer looks for actual parallel method usage:

```rust
fn alpha(x: &Alpha) -> (u32, u32) { (x.used(), x.limit()) }
fn beta(y: &Beta) -> (u32, u32) { (y.size(), y.capacity()) }
```

Each original wrapper must call two distinct zero-argument shared-receiver
methods on its sole shared nominal formal. Both methods return builtin `u32`.
Tuple position supplies the proposed operation correspondence. Matching names or
signatures without these call witnesses do not produce a proposal. Method bodies
may remain opaque: the proposal concerns a shared interface, without claiming
behavioral equivalence, purity, lifetime compatibility or a safe rewrite.

The original root file and inline modules supply a closed local declaration
index. Both enum dispatch and capability proposals borrow that same checked
index when requested together. Exact file ownership, selected-unit context,
function bytes and declaration headers must agree. Imports, expansions,
conditional namespaces, generic owners or methods, inherited methods, coercions
and ambiguous resolution remain unavailable in this initial contract. Local
method selection reuses the enum-dispatch resolver; no new compiler observation
or graph admission permission is introduced.

Two wrappers with different original owner identities can produce a two-member
proposal. Repeated calls to the same method and multiple wrappers over one owner
do not establish a capability set. If both ordered roles already name the same
exact local trait members, the pair is recorded as suppressed. Mixed, partial or
different trait relationships remain unavailable; similar trait names cannot
suppress advice. Existing legacy ignores and review entries are separate.

`capability_proposals` uses schema `rstyle.capability-proposals/1`. Its `functions`
retain evidence availability, original wrapper headers and ordered method call
and callee-header spans. Those spans index the original owning file. Proposal
`left` and `right` refer to function keys in that report; each `roles` entry indexes
the corresponding wrapper's `methods` array. Owner keys are exact analysis-local
source/module/name handles, scoped to each selected unit. They are neither stable
review identities nor executable serialized proof authority.

`--max-analysis-functions` bounds inspected owners, with an initial ceiling of
4,096. `--max-capability-pairs` defaults to 16,384 distinct-owner wrapper pairs
across units. Pair population is counted before allocating or examining pairs;
same-owner combinations require no quadratic scan. A unit exceeding its remaining
pair allowance is explicit and incomplete. Findings from other evaluated units
survive, with incomplete overall coverage. Pair counts describe known wrapper
witnesses; unavailable owners prevent a complete-scope claim.

Incomplete requested scope exits two. `--fail-on-findings` exits one for findings
when requested scope is complete. Default runs leave this consumer not requested.
Full and compact reports retain the same proposals and evidence. Explicit
`--replay-run PATH --capability-proposals` reevaluates original saved facts without
the original project files or compiler tools; serialized proposals are never
imported as authority. No legacy capability is retired by this slice.
