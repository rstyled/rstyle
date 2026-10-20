# Scoped import proof for scalar flag owners

Source-selected next practical S09 family: original regex-automata 0.4.18,
default-feature library, `dfa::onepass::Epsilons::empty` and `is_empty`.
The original nonexported tuple owner derives builtin Clone/Copy and contains
one u64. `empty` constructs zero; `is_empty(self)` compares its bits to zero.
For every u64, emptiness means exactly zero, including values outside the
42-bit encoding's ordinary representation. No implicit invariant masks bits.
Composition should preserve empty().is_empty() and caller input snapshots.

The complete original module contains unrelated generic code and macros.
The root imports a closed source macro module and publicly glob-reexports
`util::search`. Both selected functions are excluded by the current coarse
lexical expansion/import gate. Neither has an additional reported body/type
blocker. This is evidence for investigating the gate, not proof it is safe.

- `dfa::onepass::Epsilons::empty`: bytes 121083..121133, `sha256:2c852f2f5c9bea703152abd735c471145825bde2aa203a16f5df2cb2ba52ad41`; baseline **excluded**.
- `dfa::onepass::Epsilons::is_empty`: bytes 121214..121267, `sha256:ce7eb5e80c13fb040592e0fdccf4e7d240c5b88b818d369177758fb1f3ab7edb`; baseline **excluded**.

Bounded contract: use the existing independently closed, cfg-selected module
census. An original local glob may be harmless to the selected witness only
when its target module and exported names are known, have no unresolved
reexports/expansions and cannot shadow primitive, builtin derive or core/std
roots. It does not supply missing nominal witnesses. A bare macro_use module
must contain only inspected local macro definitions whose names cannot shadow
builtin derives/types; invocations remain unsupported. Unknown/external globs,
reexport chains, relevant shadowing, arbitrary macro attributes, active item
expansions, incomplete source closure and hidden drop behavior remain exclusions.
Do not widen supported body syntax, skip selected neighbors, infer Copy from a
name, or add a parallel name/provenance inventory.

Independent validation must include scalar flags with zero/high/all bits,
constructor/getter composition, imported shadowing and macro negatives,
original complete-Cargo capture, strict graph import/verification/admission and
matching. Keep the frozen selection fixed and report new/lost full-roster
admissions separately. Check rstyle/patterns and bounded runtime/memory costs.

The checked-native Result family remains a roadmap target, but its full error
closure contains macro-generated PatternIDError/StateIDError declarations.
It therefore needs broader declaration support in addition to generic length
checks, partial helper transport and residual semantics. After the observational
Try increment, prioritize this shared scope restriction before adding more
isolated syntax increments without practical gains. This does not certify the
checked readers or change their historical 0/3 result.

## Implemented boundary and activation evidence

Data-v3 3.45.0 (/58) adds `rust.scoped-nominal-imports` and
`rust.copy-value-helper-reads`. The latter connects independently proved Copy
struct formals with nonempty struct-field-only reads and scalar/unit results.
Caller substitution uses existing value snapshots and the existing
`rstyle.data:owned-receiver-reads` annotation; it does not invent external
memory reads or alias independence. Potentially panicking helpers retain their
existing caller exclusion. Shared-reference summaries retain their fast path.
No serialized provenance table, cache or source-copy format was introduced.

Independent document verification checks the required policy at both the
function's lexical scope and selected nominal declaration scopes. Old document
policies cannot acquire the new import permission or Copy input snapshots.
Historical profiles retain their closed boundary. Seven integration tests cover
complete composition, summary/source/snapshot corruption, policy downgrades,
declaration scopes, external globs, reexports, shadowing, item macros, custom
derives, absent Copy proof and destructors.

The unchanged complete default-feature regex-automata library has 2582 functions:
**14 to 59 admitted, 45 gains and zero losses**. Both source-selected Epsilons
methods gain; the other 43 gains are incidental and do not redefine selection.
Names, files, spans, source digests, compilation contexts, features, cfgs, target
and compiler commit agree with the retained baseline. Capture and assessment
complete; the zero-pair capture budget leaves whole-library enumeration and
evaluation incomplete. No whole-library matching claim follows.

The six-function immutable development corpus moves **0 to 5 admissions**, no
losses; generated Clone remains excluded. All ten development comparisons
complete. The final independent verifier imports and strictly verifies all 59
natural admissions plus the five development functions, then completes all 21
comparisons among the selected two natural and five development graphs:
**2 Accepted, 19 Rejected**, unchanged defaults and no diagnostics. Constructor
and helper graph shapes need not meet the similarity threshold. The retained
natural capture predates the final old-policy downgrade guard and diagnostic
prefix hardening; this final verification uses the final checker on its graphs.

The original complete-crate runtime oracle passes: zero, all 64 isolated bits,
all 65536 low words and shifted high words, u64::MAX, and constructor/getter
composition. This establishes independent semantic expectations, not rewrites
or held-out detector quality.

Quiescent performance measurements use one warmup and seven alternating pairs
against 745bb18. The unchanged byte-window control median is .88 to .87 seconds
(ranges .86–.90/.87–.90), median RSS 121940/122232 KiB. The unchanged cyclic
control is .60 to .62 seconds (+.02 seconds, 3.3%; overlapping ranges
.59–.61/.60–.66), RSS 111036/110928 KiB. All 14 unchanged-control matching
reports are identical. Activated flags cost .26 to .30 seconds (ranges
.23–.27/.29–.33), RSS 108384/109232 KiB, while adding five admissions and ten
comparisons. These bounded observations are not a broad performance guarantee.

Final production code passes 861 workspace tests (five existing skips),
all-target Clippy with warnings denied, doctests, formatting and whitespace
checks. Rstyle reports zero violations across 2714 functions; patterns reports
zero candidates.

Frozen replay `run-_i41wnhy` preserves all 60 selected identities and denominators:
**20 admitted /33 excluded /7 inactive**, zero gains/losses. Context counts,
source blocker combinations and outcomes are unchanged. Full rosters also retain
150/7 admitted for lz4_flex, 956/20 for regex-syntax and 226/8 for time-core,
with identical source identities and no outcome changes. Seventy documents
verify. Build evidence differences are producer revision and invocation paths
after normalization of temporary source roots; compiler/settings stay fixed.
