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
| Negative abstraction | Legacy pattern forwarder safeguards; future helper proposals | Useful repeated computation beyond bare parameter forwarding | `forward_left` / `forward_right` produces no proposal | Labels frozen; strict similarity is insufficient; do not compare proposal counts to accepted-pair counts |
| Guard clauses | Legacy structural examples; bounded new local consumer | Original branch/exit structure and opaque-boundary handling | Nested main work + trivial alternate positive; already flat and two working branches negative | Prepared; advisory only; no implementation or Go-parity claim |
| Pair differences | Native admitted pair consumer | Ordered correspondence, ambiguity and exact operation roles | `add_one` / `add_two` literal hole; swapped subtraction operands remain visible | Prepared; no WL-label bijection or equivalence assumption; unsupported/ambiguous alignment explicit |
| Legacy/new comparison | Acceptance tool | Same revision/build/scope/rule policy plus independent labels | Distinguish evaluated agreement, uncovered, intentional improvement, regression | Comparator and serial runner pass 15 unit tests and ten live label agreements; explicit contexts/rosters retained; pair/proposal quality stays separate |
| Stable review / dismissal | Legacy ignore lifecycle; later new findings | Versioned detector and locator/context with exact identity checks | Movement, relevant edit, duplicate name/context and stale-entry cases required | Deferred; report-local function handles are not persistent IDs |
| Effects / mutation / planning | Legacy rules and fix plans; later consumers | Conservative unknown-aware operation/call/alias summaries and actual edit footprints | Dedicated positive/negative/opaque-call cases required | Deferred; clone admission never grants purity or independence |

Both legacy style and patterns remain operational. The initial corpus alone does
not satisfy retirement: independent quality review, stable review identity,
configuration/suppression/exit compatibility, bounded resource evidence, broader
declared project coverage and docs/skill migration remain required. Existing Go
comparison is unavailable and unverified. Integrated gate results and commits
belong in the lead-owned progress ledger, not inferred from this matrix.
