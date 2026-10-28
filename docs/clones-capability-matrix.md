# Clones capability matrix

This is the bounded initial replacement matrix for
`CLONES_CAPABILITY_CODEX_PROMPT.md`. Acceptance labels live in
[`eval/clones-capability`](../eval/clones-capability/README.md). Labels are
source-authored requirements, not observed pass claims. The source/signature
policy is five code lines/four parameters; production defaults are unchanged.

| Capability | Existing anchor / consumer | Required evidence | Independent acceptance | Replacement state / retirement gate |
| --- | --- | --- | --- | --- |
| Function length | Legacy `rules::fn_length`; opt-in clones source rules | Exact owned source span; existing code-line policy | `length_at_limit` = 5 clean; `length_over_limit` = 6 finding | Opt-in source rule passes frozen labels and excluded-body integration; source/configuration/exit tests pass; suppression/review migration still pending |
| Parameter count | Legacy `rules::fn_params`; opt-in clones source rules | Complete signature and explicit receiver counting policy | Four parameters clean; five finding; five with closure body still finding | Opt-in rule passes frozen labels, receiver/config and unavailable-signature tests; suppression/review compatibility remains a retirement gap |
| Independent availability | New source/signature consumers | Fact availability, scope, execution and assessment kept distinct | Synthetic missing source/signature; zero parameters with opaque body; budget exhaustion | Focused and live tests preserve known-empty, unknown, unsupported, failed and budget outcomes; inactive definitions remain separately inventoried |
| Source-located strict pairs | Existing ccgraph strict matcher; clones report | Admitted original inputs, strict result and both locations | `above_left` / `above_right` accepted with binder-only change | Delivered bounded navigation: full/compact/text tests pass; strict decision unchanged; no equivalence/refactoring claim |
| Negative abstraction | Legacy pattern forwarder safeguards; future helper proposals | Useful repeated computation beyond bare parameter forwarding | `forward_left` / `forward_right` produces no proposal | Bounded helper consumer rejects forwarding and one-operation bodies; strict similarity remains insufficient |
| Guard clauses | Independent opt-in `--guard-clauses` consumer | Original branch/exit structure and opaque-boundary handling | Nested main work + trivial alternate positive; already flat and two working branches negative | Delivered bounded advisory shape; seven unit/five CLI tests and live partial-scope report; no rewrite or Go-parity claim |
| Pair differences | Native admitted pair consumer | Ordered correspondence, ambiguity and exact operation roles | `add_one` / `add_two` literal hole; swapped subtraction operands remain visible | Opt-in `--explain-pairs` implemented; 13 native unit/three real-Cargo CLI cases retain ordered roles, unsupported/ambiguous/budget outcomes and unchanged matching; no equivalence assumption |
| Parameterized helpers | Opt-in `--helper-proposals`, reusing completed native comparisons | Whole-tree u32 profile, useful operations, ordered roles and at most one literal hole | Eight frozen cases: two positive, forwarding/small/reordering negatives, call/write boundaries and nontransitive pairs | Implemented bounded advisory pairs with explicit policy, native/literal evidence and unavailable owners; no rewrite, groups or retirement claim |
| Legacy/new comparison | Acceptance tool | Same revision/build/scope/rule policy plus independent labels | Distinguish evaluated agreement, uncovered, intentional improvement, regression | Comparator and serial runner pass 15 unit tests and ten live label agreements; explicit contexts/rosters retained; pair/proposal quality stays separate |
| Stable review / dismissal | Opt-in helper review file; separate from legacy ignore | Exact versioned original free-function locator, effective context/signature/ordered expression and fresh helper finding | Movement, body/spelling/context edit, duplicate names, exact/growth/loss, collision and incomplete coverage | Bounded helper review implemented with explicit saved-run dismissal; stale reasons retained, read-only lookup, no hashes; legacy migration and broader finding families remain open |
| Saved-facts reevaluation | Explicit standalone/dependency `--replay-run` | Completed original plan/Cargo/ledger reconciliation and strict bundle import | Changed configuration without compiler/tools/source files; exact check/test population; missing/corrupt evidence rejects | Implemented bounded replay route; dependency closure checks preserved; no fresh build or changed selection |
| Effects / mutation / planning | Legacy rules and fix plans; later consumers | Conservative unknown-aware operation/call/alias summaries and actual edit footprints | Dedicated positive/negative/opaque-call cases required | Deferred; clone admission never grants purity or independence; mutation/effects/helper summaries remain required independently of saved-run replay |

Both legacy style and patterns remain operational. The initial corpus alone does
not satisfy retirement: independent quality review, stable review identity,
configuration/suppression/exit compatibility, bounded resource evidence, broader
declared project coverage and docs/skill migration remain required. Existing Go
comparison is unavailable and unverified. Integrated gate results and commits
belong in the lead-owned progress ledger, not inferred from this matrix.
