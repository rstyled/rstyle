# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-26. All P01–P22 routes
remain active in the plan; this record contains current decisions and evidence.

## Current boundary

Branch `feature/rust-feature-coverage`; Data3.119 connects original checked
wire add/mul to fresh named errors. Native no-default138/590 admissions; default
capture is complete; the selected family is excluded by imported macro source ownership. Callable arithmetic
summaries, map_err/? and broader ownership remain open. See the latest evidence
below; earlier milestones are historical. Broader P01–P22 work remains active.

Compiler nightly-2026-09-30, commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`; native aarch64 Linux,
installed x86 Linux and ARM macOS targets. Original captures use debug/default
features, lib/non-test, edition 2021 unless their family contract states otherwise.
GitHub check-runs for flags delivery `98e084c` pass Linux x86, Linux ARM and macOS;
character leaf `f61065d` and escape implementation `9967f3d` also pass all three
native CI jobs.

Preserve unrelated `crates/rstyle-pdg/src/slice_access/`, `docs/pdg-and-clones.md`
and `ferrante_toplas_87.pdf`. Lead owns shared seams, registration, staging and
commits. No new branches/worktrees. Serialize large captures, owning-unit strict
replays, performance and integrated gates; never overwrite active binaries.

## Flags: connected support and measured evidence

[Contract](extraction-profiles/look-flags-v1.md): complete checked-Copy fieldless
enums, source/discriminant proof, variants/casts, shared Copy receiver loads and
actual helper composition. Commits `4fcc007`, `0406406`, `49c7ec7`.

| Unchanged selection | Baseline → current admissions | Losses |
| --- | --- | --- |
| regex-automata 0.4.18 | 98 → 114 /2582 | 0 |
| regex-syntax 0.8.11 | 23 → 36 /956 | 0 |
| lz4_flex 0.14.0 | 8 → 8 /150 | 0 |
| time-core | 11 → 11 /226 | 0 |
| Frozen sixty | 23 admitted /30 excluded /7 inactive | 0 |

All source/build identities and denominators match. Primary 13 targets plus empty
admit; six declared negative controls remain excluded. Thirteen syntax gains are
byte-identical mirrored functions, observed incidental gains, not held-out targets.
Three extra automata gains were source-audited: StartKind's size query and Start's
explicit-load/cast/widening helpers. The optimized verifier strictly checks **169 original plus 24 development
admissions**, with **182 original and 276 development comparisons** complete. Full-library matching
remains explicitly incomplete under max-pairs zero.

Retained evidence:

- Original report `/tmp/rstyle-look-flags-natural-target/rstyle/clones/run-aOQHu7/report.json`
  (stream only); compact audit `/tmp/rstyle-look-flags-natural-roster.json`.
- Frozen run `target/hir-coverage/run-bo_05z1o`; delta
  `/tmp/rstyle-look-flags-full-frozen-delta.json`.
- Strict/matching `/tmp/rstyle-look-flags-fastpath-{strict,frozen-strict,control-strict}.log` and
  corresponding matching JSON. Original replay 121.49s /5916484 KiB; other
  complete packages 53.72s /756960 KiB.
- Independent original-library oracles pass all 262144 flag sets in both libraries:
  `/tmp/rstyle-look-flags-original-oracles-final.log`.
- Final integrated gates: 936 tests pass /5 existing skips (326.693s); doctests,
  fmt, Clippy pass; rstyle 0 /3042 functions, patterns 0 unignored. Includes
  historical-policy and receiver-signature tampering assertions. Logs
  `/tmp/rstyle-look-flags-final-{tests.log,style.json,patterns.log,clippy.log}`.

Warmup + seven alternating samples: copy median 0.62→0.62s, cyclic 0.62→0.61s,
ASCII 0.83→0.92s (unchanged admission/matching). Expanded flags 0.72→7.95s,
1→14 admissions; this is different supported work. Results retained in
`/tmp/rstyle-look-flags-performance-results.json`. A rejection-only receiver
signature fast path passes focused capture and forged-signature strict-import
tests. Review confirms unconditional independent receiver/callee verification.
Follow-up controls finished: copy/cyclic 0.62→0.62s; ASCII 0.84→0.88s; flags
0.72→7.99s. Residual ASCII slowdown remains reported. Results:
`/tmp/rstyle-look-flags-fastpath-performance-results.json`.

## Dependency frontier

| Package / owner | State and next concrete step |
| --- | --- |
| P01 character / A + lead integration | Shared integration validated: six development functions admit, seven neighboring cases exclude; literal/OR/dispatch tampering and unused-char 3.60 policy downgrade reject. 939 tests pass (five existing skips); fmt/Clippy/doctests pass; rstyle 0/3051, patterns 0. Original syntax36→38/956, no losses;57 original admissions strict-verify, three comparisons complete. Frozen60 unchanged. Original `regex-syntax::is_meta_character`; dedicated char domain and historical gating, no global widening of `scalar()`. Original numeric-set oracle passes all 1112064 valid Unicode scalars. |
| P01 escape caller / A+C+lead | Range and exact ASCII receiver leaves integrated. Complete original-copy fixture admits five/rejects five across three targets × std/no_std, strict tampering included; independent original oracle1112064 scalars/64 positives. Final focused tests pass; rstyle0/3070. Full gates pass:945 tests/5 skips, fmt/Clippy/doctests, rstyle0/3070, patterns0. Original syntax38→39/956, no losses;58 admissions strict-verify, six comparisons complete. Frozen60 unchanged. Native CI passes all three platforms. Equal-work3.60→3.62 medians: Copy0.65→0.64s, cyclic0.63→0.64s, ASCII0.87→0.87s, flags7.96→8.01s; admissions/matching unchanged. |
| P08 checked values / B + lead integration | Data3.70 connects original base64 encoded_len through checked usize add/mul, immutable Some if-let, early None return and nested result flow. Default7→8/167, no losses; six target/feature contexts strict-verify. The separate zerovec chain still needs generics/let-else/Result/?/closures/borrowed aggregates. |
| P06 borrowed result / C | Data3.69 completes original haystack → is_char_boundary → utf8::is_boundary with distinct slot reads/referent origins, saved receiver substitution and conditional checked reads. Two original gains, zero losses; all134 strict. Broader aggregate/nested-reference results remain closed; next family needs its own original source audit. |
| P05 ownership / D | Data3.68 proves selected passive Copy dependencies beside original PatternID/SmallIndex index_type_impls! invocations. All fields/source/Copy and owning-scope checks remain; direct dependency operations gain no permission. Complete automata120→132/2582, zero losses; all132 strict-verify. No census redesign justified. |
| P02 floats / lead | Data3.66 completes original k_sin with exact immutable i32 equality and complete float alternatives; libm6→7/1131, zero losses, all7 strict-verify/six selected comparisons complete. Independent original oracle5384 inputs; three-target × std/no_std composition and resealed guard/merge/source attacks pass. |

Contracts: [character](extraction-profiles/character-leaf-v1.md),
[checked consumption](extraction-profiles/saved-checked-consumption-v1.md),
[Result audit](extraction-profiles/scalar-result-frontier-audit-v1.md),
[borrowed results](extraction-profiles/borrowed-result-chain-v1.md),
[source ownership](extraction-profiles/source-ownership-frontier-v1.md),
[floats](extraction-profiles/float-kernels-v1.md).

## Resume order

Data3.74 completes the same original base64ct family under std by proving the
exact pinned alloc macro import namespace. Three targets: std **3→6/150**,
default **6/141 unchanged**, zero losses. All36 base64ct and12 contextual peer
admissions strictly replay;30 predeclared comparisons complete. Native original
length oracles pass131074 cases per feature context; private-source witnesses
are labeled separately. Frozen sixty and complete package rosters are unchanged;
all58 admissions strictly replay. Gates:1067 tests/5 skips, doctests/fmt/Clippy,
rstyle0/3398, patterns0 unignored; existing15 ignores unchanged.
Evidence: `/tmp/rstyle-alloc-*`; frozen `run-pk54tszf` versus `run-gn15_b0l`.
See the [source selection](../eval/coverage-development/base64ct-std-import-selection.json).
Full-library matching remains incomplete; broader P01–P22 work remains active.

Next bounded assignments (original first-wave workers are already integrated):

| Frontier | Readiness gate and exclusive leaf work |
| --- | --- |
| P05 external macro imports | Delivered3.74: pinned complete alloc import inventory unlocks the three original std length/getter functions. Macro bodies, allocation/drop and other import grammars remain independently closed. Lead owns source-boundary/shared policy edits. |
| P07 range/view composition | Delivered3.75 [selection](../eval/coverage-development/range-getters-selection.json): complete `Input::get_range` and `Match::range → Span::range → local From<Span>` chain. Three targets × default/no-default:139→143/2582 and85→89/590, four declared gains each, zero losses. All696 original admissions strictly replay/24 selected comparisons complete; native original4704 cases per feature context. Exact pinned non-Copy Range/no-drop authority and local trait-owner selection; only fresh whole results. No general moves/traits/storage permission. Shared-inventory performance fix adb3f5f removes repeated unit inventories; unchanged default capture182→105s, RSS6.1→1.0GiB. Full gates1090tests, style0/3470, patterns0. Broader views/element-origin work remains queued. |
| P08 concrete Result frontier | Data3.78 delivers the fresh unit-error Result chain:21 selected instances,30 complete-roster gains/zero losses across9 contexts;993 strict admissions/21 selected comparisons. Data3.79 connects original LookSet::available through exact same-error `?`:9 gains/zero losses across9 contexts,1002 strict admissions/57 selected comparisons. Next: connected Option reference/payload and concrete trait-member ownership. [Frozen selection](../eval/coverage-development/scalar-result-next-selection.json): UnicodeWordBoundaryError::check and cfg-selected helper/new; default/no-default/word-boundary contexts have independent Ok/Err expectations. Exact non-Copy nominal-error owner/drop proof plus Result construction/tag/forwarding are now connected; zerovec's broader blockers remain separate. |
| P09 loop/callee frontier | Delivered3.76 [selection](../eval/coverage-development/scalar-loop-callee-selection.json): original ryu::pow5factor_32 → multiple_of_power_of_5_32. Three targets14→15/40, exactly caller gains, zero losses; all45 contextual admissions strictly replay/6 selected comparisons Accepted. Complete alpha/transitive composition and resealed attacks pass; native private-source oracle42810 cases. Full gates1096tests/5skips, fmt/Clippy/doctests, rstyle0/3496, patterns0. General loops/recursion remain closed. |

PatternSet separately requires complete opaque Box-field/drop proof. Fold brief
evidence into semantic commits; the full P01–P22 routing remains active.

Range3.75 final frozen gate: `run-tk89n2fv` versus `run-pk54tszf`, unchanged
23/30/7 and package8/150,39/956,11/226; all58 strict. Evidence:
`/tmp/rstyle-range-frozen-{delta.json,strict.log}`. Independent semantic review
found no Range authority/freshness/historical-policy defect. Next P08 readiness
gate is independent local-owner Drop exclusion: selected census proves ownership,
not absence of destructor effects. P09 original Cargo diagnostics and strict replay are now complete under3.76;
no blanket cyclic helper permission is authorized. Data3.77 completes original
ryu::mul_shift_32 via pinned u32::MAX authority: three targets15→16/40, zero
losses;48 strict admissions/3 original comparisons,45668 private-source oracle
cases. Gates1106tests/5skips, fmt/Clippy/doctests, rstyle0/3513, patterns0;
frozen run-yaapzru3 unchanged23/30/7, all58 strict. Unsafe table wrappers remain
excluded. Evidence: `/tmp/rstyle-primitive-{evidence.log,final-*,current-*}`.

## Reprioritized frontier (2026-10-22)

Fresh Results and the selected same-error propagation chain are delivered; prioritize useful subsystems
over more numeric leaves. This order supersedes the earlier resume order; P01–P22 scope
and evidence gates remain. Rankings are source-audit judgments, not predicted
admission counts. Keep the frozen sixty and all existing development selections.

| Lane | Next connected delivery and dependency order |
| --- | --- |
| Error flow | Fresh UnicodeWordBoundaryError constructor → Result → helper → public check delivered3.78 in all nine selected contexts, with independent no-drop proof. Data3.79 also delivers original LookSet::available with exact Try/FromResidual/reflexive From evidence,9 gains/zero losses. Data3.80 connects the original Memchr/Memchr2/Memchr3 prefix methods through checked Option reference propagation, Copy-product results, exact trait-member ownership and precise Debug/import/derive proof. **Next:** original lz4_flex::block::uncompressed_size: complete no-drop error enum, differing-success Result residual, checked range/ok_or, borrowed four-byte LE decode, suffix and reference-bearing tuple result. Broader payload binding and general moves/drop remain separate. |
| Interprocedural | Bounded scalar looping-callee seed delivered3.76. Next select dependency closure for cross-package helpers or component-specific aggregate/reference arguments/results according to complete callers. Preserve panic, return and divergence independently. Further narrow numeric kernels are deferred. |
| Resolution | Address demonstrated module/import/alias/source-owner barriers, then concrete trait dispatch/selected bodies, generic instance identity/substitution, and associated types/constants/bounds. The local From<Span> delivery is a bounded precedent, not general trait support. |
| Memory/data | Select a complete common slice/view family; retain bounds, saved operands and per-element origins. Establish initialized nonescaping local storage/borrowing → alias-sensitive mutable reborrows/writes and move/initialization/normal cleanup → bounded Box/Vec/String contracts. |

Stored closures/callback environments, stateful iterator protocols/adapters/for,
relevant macro/derive/generated-source ownership, source-required standard-library
summaries and explicit observable effects remain subsequent shared frontiers.
Async/concurrency/general unsafe remain later work. Independent oracles and
resealed adversarial tests belong in every lane. Shared types/origins/summaries
remain lead-owned; serialize heavy validation. Fold concise evidence/frontier
updates into semantic commits, with no separate documentation-only milestone.

Data3.80 checkpoint: all18 original prefix instances admit across six contexts;
54 contextual gains (18 selected/36 source-audited literal siblings), zero losses.
All780 admissions strictly replay/18 selected comparisons complete. Native prefix
oracle7,966,720 cases plus5120 sibling queries;2564 constructors/context,
no-default constructors only. Frozen run-etikyyi5 unchanged23/30/7, all58 strict.
Gates1153tests/5skips, doctests/fmt/Clippy, rstyle0/3792, patterns0;15 ignores unchanged.
Next assignments: error/data lane freezes lz4_flex0.11.6::uncompressed_size with
complete enum/drop and differing-success Result obligations; interprocedural/
resolution lane audits time0.3.47→time-core0.1.8 exact seven-function dependency
closure; performance lane removes demonstrated immutable-source identity cache
bypasses. Shared production interfaces stay lead-owned until readiness contracts
and exact blockers are established. No broad trait, ownership or coverage completion.

## Calendar and next instance gate

Data3.82 connects all eight original calendar helpers in six fixed contexts,
18 contextual gains/zero losses, with strict closure replay and complete153-pair
assessment per context. [Contract](extraction-profiles/cross-unit-scalar-helpers-v1.md).
Frozen source identities/denominators and23/30/7 remain unchanged; all58 contextual
frozen admissions strictly replay. Final quality gates are recorded in the roadmap.

The [Input instance selection](../eval/coverage-development/input-instances/README.md)
freezes original regex-automata0.4.18 `Input::new<H>` and blanket `From<&H>::from`
for `[u8]`/`str`. Data-v3 now observes concrete requested/selected call arguments,
structural type/definition identities and source anchors, without altering generic
bodies, policy capabilities or admission. Six-context Cargo captures record24
observations with zero admission gains/losses; all six owning bundles strictly
import. The native oracle passes132,736 constructor observations per feature
context plus stateful/static-origin/panic controls. Region erasure is not lifetime
proof; foreign metadata is not the owner's original HIR/typeck.

Audit found a smaller connected next delivery: original num-traits0.2.19
`clamp`, `clamp_min`, `clamp_max`, each at i32/u32. Its separate
[source-first selection](../eval/coverage-development/clamp-instances/README.md)
passes272,852 scalar native calls in each of four feature/assertion contexts,
including4921 expected panics in each assertion-enabled context and custom trait
order controls. This avoids Input's owned-reference/mixed-enum/cleanup blockers;
it does not replace the Input selection or establish instance admissions.

Data3.83 now connects the six original clamp scalar instances in all four native
feature/assertion contexts. All 24 strict instance sidecars admit; all 48 contextual
callers strictly replay and all 264 caller-pair evaluations complete. Original
2,212/1,965 owner rosters remain unchanged, with generic definitions still excluded.
The bounded contract and reproducible commands are in the clamp selection README.
Fresh frozen run `run-0qhwddyy` preserves all60 source identities/denominators and
23/30/7 outcomes against `run-etikyyi5`, with116 schema-validated graph documents.
This reproduction is not an additional strict semantic replay or native macOS/x86 claim.

Next dependency frontier:

| Assignment | Readiness gate |
| --- | --- |
| Concrete reference/aggregate instances | Preserve the four original Input instances and oracle; prove body/type substitution, static/blanket impl ownership and lifetime obligations before extending scalar-only capture |
| Exact trait summaries | Prove the actual [u8]/str AsRef implementations and their returned origins; stateful, static-origin and panicking implementations stay negative controls |
| Complete result ownership | Audit every Input/Anchored field/variant, PatternID/SmallIndex, relevant derives/imports and no-drop transfer; existing getters and one chosen variant do not certify the constructor |
| Component summaries and performance | Bind each foreign returned component to exact source operands and lifetimes. Measure complete captures/replays; retain XXH3 byte seals and typed keys, with no serialized cache keys or new SHA format |

These are readiness-gated assignments, not four concurrent production edits.
Shared registrations, instance keys, origins and summaries remain lead-owned.
Broader P01–P22 work remains active; scalar instance delivery does not establish
general traits, reference transport, mutation/cleanup or completion.

Reference-owner diagnostic gate: all eight original Input new/From observations
now retain inherited binders/clauses, absolute anchors, selected AsRef receiver
adjustments and exact constructor linking; rosters remain2,582/590. Evidence:
`/tmp/rstyle-input-reference-owners-final/summary.json`. Next integrate an independent
reference projection witness against the full canonical owner, accounting for
the exact deferred AsRef source finding without changing raw diagnostics. Reuse
the complete original result tree, declaration origins and71-file source census
for actual product cleanup; modeled cleanup witnesses alone do not establish it.
Then connect outer From trait/impl proof, owned/component result transport and
foreign summaries. No new capability, SHA operation or serialized lookup key.


Data3.84 integrates bounded local component summaries and the actual outer From
association proof. Full original constructor cleanup and selected conversion/body
proof are available; canonical opaque compiler IDs need no new capture format.
The next connected activation retains a private checked reference owner, covers
only its exact deferred AsRef finding, lowers that proved operation with saved
operand origins, and proves concrete Input slots plus the complete mixed Anchored
type. Then connect owning artifact/caller links and foreign component summaries.
Current mechanism fixtures do not establish original Input admission. Keep the
error-flow frontier (original lz4_flex::uncompressed_size) and broader memory work
active after this chain; no additional narrow numeric kernels are prioritized.

The next milestone must deliver original constructor admission through this
connected chain, rather than another diagnostic-only increment. Reassess if
activation uncovers a new foundation that cannot finish the declared family.

Data3.85 delivers the original `Input::new::<[u8]>` through checked construction,
strict sidecar replay, producer admission and complete identity matching in both
native feature contexts. Each unchanged four-instance selection gains1/4, loses0;
original rosters remain2,582/590. The other three instances remain unsupported.
Fresh captures/probes: `/tmp/rstyle-385-input-capture` and
`/tmp/rstyle-385-fresh-{default,no-default}-activation.log`.
No new SHA operation or serialized lookup key. The whole family remains incomplete.

| Next assignment | Readiness gate and exclusive leaf work |
| --- | --- |
| String byte-view origins | Delivered3.86: original str constructor admits and strictly replays in both native contexts. Exact `str → [u8]` origin typing, independent component expectations and malformed projection/type/formal negatives; both constructors now admit, both From instances remain unsupported. |
| Original From activation | Association proof exists. Materialize the complete checked forwarding body and exact constructor dependency at identical substitution; derive each returned component and replay both proofs. Separate token/site proof from constructor body assumptions. |
| Actual reference caller links | Requires production proof-input inventory and typed instance key, exact compiled-artifact/definition association and foreign reference/component summaries. Diagnostic observations alone grant no linkage. Follow the preceding activation boundaries. |
| Next connected error/data chain | `uncompressed_size` was delivered in3.81 and remains a regression control. Source audit selects base64ct1.7.3 `Line::slice_tail → len`: scalar Result payload propagation, exact fieldless error ownership/imports, checked suffix get and shared Line field origins. Freeze expectations/build contexts before evaluation; public Decoder callers still need loops/mutation. |

Coordinate shared edits serially; leaf audits/oracles may proceed independently.
Keep the existing four-instance denominator and the separate frozen sixty intact.

Data3.86 checkpoint:1,427 tests/five skips, doctests/fmt/Clippy pass;
rstyle0/4,605, patterns0/15 unchanged ignores. Fresh frozen `run-6jdco7wi`
preserves all60 identities/denominators and23/30/7 against `run-mj68hs_m`.
Native oracle132,736 observations/context; both constructor identity comparisons
and strict graph attacks pass. Evidence: `/tmp/rstyle-386-*`. Next connect the
two original From callers and their constructor dependency; no ordinary foreign
caller admission or full-library matching completion is claimed.


Data3.87 connects the two original From bodies to their exact checked constructor
at the same [u8]/str substitution. The unchanged native selections gain2 each,
lose0, and reach4/4; rosters stay2,582/590. Strict forwarder replay reconstructs
both proofs and the sole typed dependency/site; producer admission, independent
component origins, complete identity matching and semantic attacks pass in the
fresh Cargo owners. No raw dispatch is invented. Typed nested sidecars remove JSON
round trips; checked immutable owner sessions are reused. No new hash/key format.
Evidence: `/tmp/rstyle-387-*`.

Foreign Input linking remains a foundation lane: production reference inventories,
loaded artifact/selected-impl evidence and caller component authority are absent.
Source audits found no ready new foreign scalar-instance caller in the cached
corpus; existing calendar/clamp chains remain controls. Prefer demonstrated
error/ownership prerequisites over another isolated min/max leaf.

Data3.87 checkpoint:1,442 tests/five existing skips, doctests/fmt/Clippy pass;
rstyle0/4,641; patterns0/15 unchanged ignores. Fresh frozen `run-26yohht4`
versus `run-6jdco7wi`:60 unchanged identities/denominators,23/30/7,0 changes,
116 schema-checked documents (not an additional frozen semantic replay).
Fresh full Input probes:136.63/20.22s default/no-default,1,607,372/281,300KiB
peak RSS; workload includes both From instances, so no speedup claim. Native
Input oracle132,736 observations/context and Line witness1,209,654/context
have0 mismatches; the latter still has no whole-original admission evidence.


Data3.88 delivers original `Line::slice_tail → len`: each unchanged default/std
selection on ARM Linux, x86 Linux and ARM macOS moves1/2→2/2, gains1/loses0;
complete rosters141/150. Strict replay of six complete bundles,12 selected identity
comparisons and48 resealed attacks pass. Public Decoder callers and borrowed
Result helper summaries remain separate. Full gates:1,466tests/five existing
skips, doctests/fmt/Clippy pass; style0/4,709, patterns0/15 unchanged ignores.
Frozen `run-9bjlp2ny` preserves all60 identities/denominators and23/30/7 against
`run-26yohht4`, with116 schema-checked documents. Evidence: `/tmp/rstyle-388-*`.

Next source audit selects original `LineReader::new_unwrapped`, excluded in all
six retained complete contexts after expectations were frozen. It returns
Err(InvalidLength) iff bytes are empty, otherwise a fresh non-Copy product with
the exact backing and `line_width=None`. Complete caller `new_wrapped` additionally
needs aggregate Result `?`, local field updates and fresh return; public Decoder
constructors add generic decoding, loops, arrays/byte writes and iterator state.

| Next assignment | Readiness gate and exclusive leaf scope |
| --- | --- |
| Nominal ownership | Reuse complete reference-product no-drop proof; select Result success owner and prove scalar Option fields. Preserve Clone/Iterator/import/macro/Drop neighbors; no Copy permission. Nominal resolution subtree only. |
| Capture/types/freshness | Capture bounded nested Option<scalar>; combine exact Copy error and fresh product Result witness. Separate boundary proof from full body walk to avoid constructor-proof recursion; reject stored/reused/moved products. Driver type capture and concrete Result leaves only. |
| Guarded origins | Prove Ok product component origins and independently validated empty Err paths, using saved operands and typed selectors. Keep Result helper export closed until consumers prove branch extraction; origin source/graph leaves only. |
| Integration/performance | Lead owns registrations, source/frontend gates, independent oracle/Cargo tests, captures and all final gates. First remove inventory harness full-report duplication via existing compact navigation and complete bundles; preserve source correspondence and unchanged-selection parity. |

Source/expectation audit is complete; product activation has not started. Do not
invent generic instance authority for this nongeneric constructor. No new SHA
operation or serialized lookup key. Broader P01–P22 scope stays active.

Inventory performance boundary: existing compact reports and one-bundle-at-a-time
projection replace full JSON duplication; assessment v4 retains complete evidence
navigation. Fresh `run-gp8bnmsf` preserves all60 identities/outcomes/denominators
and116 graph schema checks versus `run-9bjlp2ny`. Regex-syntax stdout1.85GB→3.99MB;
assessment115.36MB→0.70MB. All21 Python checks pass; no Rust semantic change or new
hash. Single fresh run50.36s/1,235,844KiB max RSS; no controlled speedup claim.
Next activate the source-frozen LineReader ownership bridge with coordinated
nominal/type/origin interfaces above.

Data3.89 connects original `LineReader::new_unwrapped` through complete fresh
non-Copy product Result ownership, scalar Option cleanup and guarded origins.
Each unchanged six-context selection gains1/loses0 (0/1→1/1), rosters141/150.
Six strict owning-bundle replays, six identity comparisons and36 semantic attacks
pass; ARM Linux default/std oracle9,234 cases each has0 mismatches. Full gates:
1,483 tests/five existing skips and doctests, fmt/Clippy; style0/4,743,
patterns0/15 unchanged ignores. Composition0.878s with prebuilt driver; no new
production hash or serialized key. Evidence: `/tmp/rstyle-389-*`.

The next source audit identifies the complete `new_wrapped` chain, rather than
opening helper export alone. Expected: InvalidLength iff width<4 or bytes empty;
otherwise exact backing and Some(width). Three connected prerequisites remain:
guarded product Result summaries, same-product same-error `?`, and linear local
ownership with a Copy field update followed by one whole move. Existing selectors
and no-drop proof do not establish any of these by themselves.

| Next assignment | Readiness gate / exclusive leaf scope |
| --- | --- |
| Lead integration/oracle | Freeze separate unchanged `new_wrapped` selection and independent width/empty/order expectations; confirm captured MIN_LINE_WIDTH and exact generated Try selections. Own shared registrations and all gates. |
| Callable boundary | Specify guarded Ok component/value summary and explicit Err alternative using existing checked call links; preserve helper panic/divergence/error evidence. Helper-summary leaves only after interface freeze. |
| Ownership flow | One initialized nonescaping no-drop owner, Copy scalar/Option field replacement and exactly-once terminal move. Reject aliases/borrows, partial moves, reference-field writes and unsupported merges. Dedicated local ownership leaves. |
| Guarded origins | Substitute exact helper success components under the checked continuation; preserve untouched backing across the local aggregate transition. Origin source/graph leaves after transfer interface freeze. |

Status: next caller source-audited, capture/oracle readiness checks pending;
production activation not started. This has higher demonstrated caller value
than more leaf operations, with moderate/high integration uncertainty across
summary/propagation/local-value interfaces. Retain the small six-context Cargo
validation set and serialize builds/replays; no general ownership architecture is
required by the declared single-local family. Broader P01–P22 scope stays active.
Frozen checkpoint `run-49q4l17t` versus `run-gp8bnmsf`:all60 identities/outcomes,
reasons/build settings and denominators unchanged,23/30/7,0 gains/losses;
116 schema-checked documents and21 Python tests. No additional frozen strict
semantic replay claim. Assessment parity: `/tmp/rstyle-389-frozen-parity.json`.


Data3.90 connects original `LineReader::new_wrapped → new_unwrapped` without
changing earlier selections. All six target/feature contexts gain1/lose0,
rosters141/150 unchanged; six strict replays/identity comparisons and108 resealed
attacks pass. Original-source oracle64,638 cases/default and/std has0 mismatches.
Final gates:1,497 tests/five existing skips, doctests/fmt/Clippy, style0/4,843,
patterns0/15 unchanged ignores. Composition1.646s; no new hash or serialized key.
Frozen `run-dm2kys18` versus `run-49q4l17t`:all60 identities/outcomes/blockers/build
settings and denominators match,23/30/7,116 schema checks (not new strict replay).
Evidence:`/tmp/rstyle-390-*`; [contract](extraction-profiles/linear-product-result-flow-v1.md).

Next frontier is source-audited, not implementation-ready. Guarded Result summaries
do not authorize plain foreign Input products. Prefer a bounded resolution chain
before another getter; freeze exact expectations before measuring admission.

| Lane / exclusive scope | Next target and readiness gate |
| --- | --- |
| Lead integration/oracle | Freeze separate Match constructor selection, concrete build/substitution identities and independent value/panic oracle; own shared policy/types/origins and gates. No promised upstream search gains. |
| Resolution/callables | Original regex-automata0.4.18 `Match::new<Span/Range<usize>>` plus exact Into→From bodies. Prove complete Match/PatternID source/Copy ownership, actual blanket implementation, once-only conversion, assertion/panic and full scalar result. Instance/dispatch leaves only after interface freeze; Range consumption is an extra obligation. |
| Error/payload | Audit time0.3.47 ComponentRange→TryFromParsed→inverse payload match. Actual fields are static str+bool; require static origins, complete mixed enum/unit error, concrete trait selection and saved match payload proof. No parser-caller gain claimed; error/pattern leaves only after capture readiness. |
| Memory/data | Frozen safe lz4_flex0.14.0 `backtrack_match`: mutable scalar formal locations, alias-sensitive loop memory/read-write/post-state and panic contract before helper export. Then base64ct `Line::take` adds split_at, tuple binding, static empty slice and descriptor writes. No public compressor/decoder gain claimed; memory leaves only after contract freeze. |

Bounded source audits:`/tmp/rstyle-390-{resolution,error,memory}-frontier.md`.
All three candidates have unresolved capture/interface questions; reassess if a
large prerequisite chain yields no useful originals. Preserve all earlier owners,
foreign/linking restrictions and frozen membership. Native x86/macOS execution,
full-library matching, general cleanup and broader P01–P22 routes remain active.

Match readiness now has a separate frozen two-instance
[development corpus](../eval/coverage-development/match-construction/README.md).
Native default/no-default original-library oracle:11,448 successes,11,024 exact
assertion panics and3 conversion/drop adversaries per context,0 mismatches;
Range:Copy independently fails. Native no-default owning diagnostics retain590
original definitions and both16-node concrete observations, including the selected
local From body. No new admission capability or gain is claimed. The enriched
roster already captures complete Match/PatternID/SmallIndex/Span trees and Copy
origins; activation still needs source closure, verified blanket Into delegation,
full scalar-product composition and separate affine Range consumption. Bare
diagnostic source_scan_complete=false remains preserved, not treated as clean.
Evidence:`/tmp/rstyle-391-*`. Data3.90 native Linux ARM/x86 CI passed; macOS Tests
failed and its failing-test log remains required (unauthenticated log API403).
Preparation checkpoint:1,499 tests/five existing skips and doctests pass;
fmt/Clippy pass; style0/4,878, patterns0 with15 existing ignores unchanged.
This is capture readiness, not strict instance replay/admission or matching.

Data3.91 private Span checkpoint (base `8a60512`): original `Match::new<Span>`
passes owning production, serialized verification, strict matcher admission and
sidecar replay on each retained ARM Linux/x86 Linux/macOS capture (590 original
definitions each). Raw generic/projected facts and Range remain unavailable.
Both selected instances remain in scope; ordinary caller/frozen admission deltas
are unmeasured. Foreign capture replay ran on ARM, not native foreign execution.
Gates:1,541 tests pass/five existing skips in128.423s plus doctests; fmt/Clippy,
style0/5,075, patterns0 with15 unchanged ignores, diff check pass. Source-only
oracle checks pass; this checkpoint did not rerun the native value oracle.
Evidence:`/tmp/rstyle-394-{suite,style,patterns,clippy}-final.log` and
`/tmp/rstyle-394-strict-probe-{native,x86,macos}-final.log`.
Next: Range's consuming formal/ordered endpoints and exact original From body,
then an owner-backed Into-to-From helper dependency using existing strict replay.
Adapter audit:`/tmp/rstyle-394-range-dependency-adapter.md`; shared registration
and owner integration stay lead-owned. Fresh six-context capture and complete
selected matching remain pending. [Contract](extraction-profiles/product-constructor-instances-v1.md).

### Data3.92 — owning Range constructor and exact local conversion dependency

The unchanged two-instance Match selection now has both private owning instances:
`Span` and `Range<usize>`. Range consumes its formal once, reads the original
From body's ordered usize endpoints and binds the original Into site to that
certified callee in its retained complete owner. It remains non-Copy/no-drop;
no generic trait dispatch or general cleanup permission is inferred. The source
receipt binds the exact static trait method without relaxing neighboring-item
or callable ownership proof. Diagnostic-to-original joins allow checked type
normalization and source-equal redundant parameter spelling only.

Three retained ARM/x86/macOS 590-definition contexts pass original From strict
admission and both constructor instances' production, serialized verification,
strict admission/replay, namespace negatives and three resealed CALL attacks.
Selected owning instances gain1/lose0 against3.91; ordinary caller and frozen
counts are unmeasured here. No fresh capture/native foreign run is claimed.

Instrumented pre-session-reuse probe samples: native21.103s, x8620.475s,
macOS20.735s. Native source.snapshot_hash events total0.651s; unit.capture9.889s.
Stages are nested, so totals cannot be added. These are diagnostic samples, not
controlled performance comparisons. Reuse one existing checked replay session
for both instances and clone typed test documents directly before resealing;
no new SHA operation, serialized lookup key or speculative cache.

Frontier: fresh six-context original captures → strict two-instance replay and
complete selected pair evaluation → reassess the next complete caller chain.
Broader error-flow/callable/resolution/memory priorities remain as recorded above;
this checkpoint does not complete the family or P01–P22. Evidence:`/tmp/rstyle-395-*`.

Checkpoint gates: fmt and warning-denied workspace/all-target Clippy pass;
1,559 workspace tests pass/five existing skips (128.631s), plus workspace doctests.
Style0/5,163; patterns0 with15 existing ignores unchanged. Source-only oracle
passes; no fresh native value-oracle or frozen reassessment is claimed.
Final integrated replays also pass all three retained contexts: native16.776s,
x8616.881s, macOS17.032s (single diagnostic samples). Native frontend observations
fall5,226→4,138 after replay-session reuse; hashing remains0.499s. This demonstrates
removed repeated work, not a controlled speedup estimate.


### Match composition performance — unchanged Data3.92 support

Fresh default/no-default captures retain590/2,582 definitions on ARM Linux,
x86 Linux and ARM macOS; both private instances and all six original/replay
pairs pass strict verification in every context. Fresh native original-library
oracle:22,475 cases per feature context, zero mismatches. Foreign captures do
not establish native execution; ordinary generic callers remain closed.

Source correspondence now uses an interval index, lexical function ownership
uses the existing module/span index, and completed immutable nominal-source
failures remain errors in their existing verification slots. Frontend failures
remain retryable as helper certification progresses. Nominal proof work falls
1,532→482 checks and7.111→3.006s in diagnostic runs. Controlled native no-default
warmup plus three alternating equal-work samples:16.27–16.79s before versus
12.14–12.65s after; peak RSS291,936–292,000 versus289,132–289,292KiB. Complete
matching output is identical. Indexes alone produced only small gains.
Evidence:`/tmp/rstyle-396-{bench,final-contexts}.json` and associated logs.

Routine source/manifest/declaration SHA remains a migration task: reuse existing
XXH3 for new consistency claims, explicitly verify legacy SHA identities and
reviewed library pins, and preserve r0's historical conformance requirements.
The audit found no cryptographic requirement for routine new source checks.

Checkpoint gates:1,574 tests pass/five existing skips plus doctests; fmt and
warning-denied Clippy pass; style0/5179 and patterns0/15 unchanged ignores.
Gate logs:`/tmp/rstyle-396-*-final.log`. Frozen reassessment remains pending.


### Routine source checksums — unchanged Data3.92 support

Fresh Data-v3 function claims and fresh manifest/file/declaration claims now use
existing XXH3. Strict checks dispatch by the explicit scheme; malformed/unknown
claims, missing bytes, changed ranges and contradictory linked records remain
errors. Actual immutable owners retain lazy SHA verification for old claims;
r0 function conformance and reviewed library/frozen SHA pins remain unchanged.
[Format boundary](performance/unit-build-contexts.md#routine-source-consistency-subsequent-data392-increment).

All1,577 tests/five existing skips and doctests pass; fmt/Clippy pass,
style0/5,189 and patterns0/15 unchanged ignores. Python inventory tests23 pass.
Unchanged frozen sixty:23 admitted/30 excluded/7 inactive, gain0/loss0 versus
`run-dm2kys18`; fresh evidence:`target/hir-coverage/run-d4nnwn0k`. This inventory
verifies capture/schema/correspondence, not strict matching completeness.
Retained SHA and fresh XXH3 native590-definition Match owners both pass strict
constructor verification/replay and all six selected matching pairs, with
identical decisions/scores. Diagnostic replay samples12.16/12.05s do not establish
a controlled speedup. Evidence:`/tmp/rstyle-397-{gates,frozen-comparison,matching-results}.json`.
No new admission capability. Ordinary generic callers and the mutable backtracking
blocker chain remain outstanding; the P01–P22 goal stays active.

### Mutable scalar formal evidence — backtracking prerequisite

Fresh Data-v3 capture observes `&mut usize` formals without activating mutable
types or admission. Independent source/HIR verification checks five parameters,
eight ordered accesses and ten conservative alias pairs for the unchanged safe
`lz4_flex 0.14.0::block::compress::backtrack_match`. Historical absence remains
absence; other mutable types, escapes, aliases, rebindings and reborrows stay
unsupported. HIR size/identity checks precede source indexing. The existing source
interval index is reused; no serialization key or SHA computation is added.
[Bounded contract](extraction-profiles/mutable-scalar-formals-v1.md).

All1,578 tests/five existing skips and doctests pass; fmt/Clippy pass,
style0/5,237 and patterns0/15 unchanged ignores. The composition test takes0.294s
in this diagnostic run, not a controlled speedup estimate. Native private-body
oracle:48,856 cases, zero mismatches (38,363 normal/10,493 bounds panics).
Evidence:`/tmp/rstyle-398-*-bounded.log`.

Fresh frozen sixty retain23 admitted/30 excluded/7 inactive, gain0/loss0 and all
identities/denominators/outcomes unchanged versus`run-d4nnwn0k`. New capture:
`target/hir-coverage/run-gdz69t4m`. Strict original150-definition lz4 owner
readiness passes; the selected full original remains excluded. Evidence:
`/tmp/rstyle-398-{frozen-comparison.json,readiness.log}`. This supplies prerequisite
evidence, not admitted pair or matching coverage.

Next activation must connect formal locations, cyclic RAW/WAR/WAW dependencies,
weak updates, and normal/failure post-state through emission and strict replay.
The schema supports exceptional exits, but current producer emission/verification
assumes one normal exit. Under possible aliases, the first cursor store can alter
the second compound load and cause a later checked failure; an attached panic
annotation cannot preserve that stored state. The oracle's distinct cursor objects
do not discharge captured alias uncertainty. D20/D21 and the full family remain
incomplete; do not continue with unrelated capture-only increments.

### Data3.93 — complete bounded mutable backtracking

Connects unchanged safe `lz4_flex0.14.0::backtrack_match` through explicit scalar
referent loads/stores, alias-conservative cyclic RAW/WAR/WAW and terminal checked
failure exits with complete per-root post-state. A source/CFG document verifier
independently reconstructs memory flow; resealed attacks and local-ID renaming
exercise strict replay. Direct mutable callees (including unused named formals)
cannot export read-only summaries; their callers remain excluded.

Fresh frozen inventory `target/hir-coverage/run-ng8slil8`: all60 identities and
denominators unchanged;24 admitted/29 excluded/7 inactive, gain1/loss0 versus
`run-gdz69t4m`. The only change is the selected safe backtracking function.
Full lz4 owner:9 admitted/141 excluded versus8/142. Native strict original/replay
matching:one complete Accepted pair, score1, retained150-definition owner.
Native private-body oracle:48,856 cases, zero mismatches. Its distinct cursors do
not prove captured disjointness; the abstract alias model retains failure after
the first store. Evidence:`/tmp/rstyle-399-{natural-strict.log,frozen-comparison.json,oracle.log}`.

Next resolution target is source-audited `Match::must<Span/Range<usize>>`, not
implementation-ready: it also needs the generated PatternID→SmallIndex fallible
constructor chain, error/expect ownership, actual instance links and evaluation
order. Source audit:`/tmp/rstyle-399-next-resolution.md`. Lead owns shared interfaces;
resolution owns instance-link leaves, error lane owns selected error/expect proof
and oracle, source lane owns relevant generated-callable capture proof, memory
lane audits original `Line::take` before borrowing/storage/view activation.
No caller gain is attributed to private owning-instance sidecars. Plain stores,
reborrows, local reference storage, stateful helper summaries and general cleanup
remain outside this milestone. Broader P01–P22 work stays active.

Checkpoint gates:all1,601 workspace tests/five existing skips and doctests pass;
fmt and warning-denied Clippy pass;style0/5,383 and patterns0/15 unchanged ignores.
Logs:`/tmp/rstyle-399-final4-{clippy,style,patterns}.log` and
`/tmp/rstyle-399-final5-{fmtcheck,tests}.log`. Routine consistency uses existing
XXH3; this activation adds no SHA or serialized lookup keys. Worklist/state,
root/access and dependency budgets bound memory analysis. Diagnostic strict CLI
compositions take2.18s/4.48s with a prebuilt driver; no controlled speedup claim.

Fresh complete unchanged150-definition owners in all three declared default-feature
contexts (ARM Linux, x86 Linux, ARM macOS) now pass strict source/build/access and
producer/document checks, strict original/replay admission and complete selected
matching:one Accepted score1 pair per context. Evidence:
`target/backtrack-match-399/{aarch64-unknown-linux-gnu,x86_64-unknown-linux-gnu,aarch64-apple-darwin}/`
and`/tmp/rstyle-399-final5-cross.log`. Native foreign-target execution remains
pending; cross-target metadata capture does not claim it.


Time error-conversion readiness at `b35a98a`: freeze four unchanged original
constructors/conversions in a separate [selection and oracle](../eval/coverage-development/time-error-conversions/README.md).
Native default+parsing/no-default+parsing captures retain1,288/1,080 complete
raw definitions. Both selected conversion members now retain exact core trait,
local impl/member/source, signature and nominal DifferentVariant Error projection
observations. A diagnostic-only recursive Copy/static-str source probe preserves
whole-source cleanup obligations; neither increment grants admission or a new
profile capability. Original public conversion oracle7 cases/context and copied
private-constructor diagnostic15/context pass; the latter is not natural
constructor execution. No SHA or serialized cache key is introduced.

The full blocker chain is larger than the four bodies suggest: foreign anonymous
trait glob `num_conv::prelude`, generated namespace `time_core::convert` reexported
by time, lexical enum/parent globs, time's multi-arm/generated macro closure,
then complete mixed Copy/Result payloads, guarded reference origins and lexical
trait-body/call proof. Existing snapshots contain no foreign export footprint.
The probe rejects the unresolved neighboring date-module glob rather than
omitting it. Reassess this source prerequisite before further observation-only
increments; do not claim a cheap four-function unlock. Strict admission/matching,
foreign-target capture/native execution and natural gains remain pending. Frozen
60 membership is unchanged;24/29/7 is the last measured result, not a rerun here.
Evidence:`target/time-error-conversions-400/`, `/tmp/rstyle-400-time-oracle.log`,
`/tmp/rstyle-400-time-readiness-before.json`; full contract/blockers are bundled
with the selection rather than another extraction-profile report.

Checkpoint:1,609 workspace tests/five existing skips plus doctests pass
(`/tmp/rstyle-400-tests.log`, nextest129.483s). Final diagnostic predicate
simplification separately passes all six source/adversarial tests. Warning-denied
Clippy and fmt pass; style0/5,410; patterns0 unignored/15 unchanged ignores.
Logs:`/tmp/rstyle-400-final-*`. No natural admission gain is claimed. Before more
time metadata work, audit ordinary bidirectional Span/Range conversion ownership
and its natural callers against the already proved private Match dependency;
retain time's larger source prerequisite as a separate, explicit frontier.

### Data3.94 — original Default initialization composition

From `6a7df75`, connects unchanged adler2 2.0.1 `Adler32::new → Default::default`
using exact reviewed core trait/member selection, independently captured raw call
inventory and local impl/source/signature proof. Effects and returned fields come
from the original callee body and existing zero-argument Copy-product summaries.
[Selection, bounded contract and independent oracle](../eval/coverage-development/adler-default/README.md).

Fresh complete std/no_std owners retain17/16 definitions in ARM Linux, x86 Linux
and ARM macOS. Each context gains exactly `new` and `default` (2→4 admissions),
zero losses; source/build selection identities match the3.93 baseline. All six
ordinary pairs per owner are evaluated;12 strict original/replay identity pairs
are Accepted, score1. Native oracle passes8 original public cases,2 copied-private
field witnesses and3 dispatch controls per feature context. Foreign native
execution remains pending. Frozen `run-bbmmkg_8` vs `run-ng8slil8`:all60 identities,
outcomes and denominators preserved,24 admitted/29 excluded/7 inactive;118 schema
checks, gain0/loss0. This inventory assessment is not a new strict semantic replay.

Checkpoint:1,618 tests/five existing skips plus doctests; fmt and warning-denied
Clippy pass;style0/5,449;patterns0/15 unchanged ignores. Explicit direct-document
and strict-producer3.93 downgrades reject owning members and callers. Resealed raw
inventory, dispatch, source/type and shadowing attacks remain closed. Evidence:
`/tmp/rstyle-401-final{4,5}-*`, `/tmp/rstyle-401-final-{captures,strict}.json`,
`/tmp/rstyle-401-frozen-comparison.json`, `/tmp/rstyle-401-adler-oracle.log`.

Performance correction: the historical22.312s six-capture stored-slice composition
spent0.03291s hashing sources; SHA was not the principal cost. Fresh fixed-binary
samples8.953–9.008s and current full-suite10.126s pass. Workload includes six Cargo
captures, verification and matching; these observations do not isolate a speedup.
Routine consistency remains XXH3; historical SHA/pins retain compatibility.

Next connected frontier is SmallIndex checked construction, then generated
PatternID construction, advancing actual Match::must callers. Coordinate four
scopes: error lane freezes original public API expectations/non-Copy error shape;
resolution lane proves exact local Usize and selected core usize→u64 conversion;
source lane proves generated callable/error ownership and ordered map/map_err;
lead owns MAX/Result/summary/policy interfaces and integration. Full known chains
must pass readiness before activation. Hasher::finish is deferred: no higher-level
natural caller was found. Memory lane retains Line::take as unready: local storage
alone cannot discharge split_at, static empty origin, mutable descriptor writes,
tuple flow and stateful helper summaries. Audits:`/tmp/rstyle-401-next-{match,hasher,memory}.md`.

Next [checked-index source selection/oracle](../eval/coverage-development/checked-index-construction/README.md)
freezes six ordinary functions, three generated templates (not emitted identities)
and full known blockers before admission evaluation. Actual public SmallIndex and
PatternID oracle passes8,202 cases in each of four feature/debug contexts; copied
private Usize controls are diagnostic only. Retained artifacts establish no current
complete-owner admission baseline: old complete capture predates instance evidence,
newer diagnostic capture is incomplete. MAX observation, debug total-conversion
proof, non-Copy struct errors and generated callable/constructor callback ownership
remain readiness prerequisites. Source/error lanes retain disjoint leaves; lead
must schedule a current complete-owner capture before production activation.


Data3.95 source increment (P05/P22 prerequisite for checked index): original
ordinary Copy-owner receipts now feed declaration and function namespace checks.
They retain exact method/impl/interface/build/source ownership and exhaustive
neighbor expansion proof; private-instance result receipts keep their existing
invariant. No generated callable, purity, alias or cleanup shortcut is granted.
Fresh native ARM Linux no-default regex-automata owner:590 unchanged definitions,
103→109 admissions, six gains/zero losses. Preselected six gain only new_unchecked
and as_usize; incidental getters/as_i32/one_more account for the other four.
All six gains strictly verify/admit/replay and identity-match Accepted score1;
complete-library matching remains budget-incomplete. Cargo composition and3.94
policy downgrade tests pass. The replay example batches selections over one
strictly imported original/replayed owner, avoiding six repeated full imports.
No SHA computation, serialized lookup key or new cache substrate was added.
Evidence:`/tmp/rstyle-403-{native-delta.json,final-strict.log}`. Final checkpoint:
1,624 tests/five existing skips plus doctests pass; fmt and warning-denied Clippy
pass; style0/5,480 and patterns0/15 unchanged ignores. Direct/generated primitive
shadows and unresolved generated globs reject; ordinary lexical import checks
remain mandatory. Strict replay finishes six identity comparisons and all15
original pairs (10.35s); native oracle passes8,202 public cases in each of four
feature/debug contexts. Frozen `run-d3tge24c` vs `run-bbmmkg_8`: all60 identities,
denominators and outcomes unchanged,24 admitted/29 excluded/7 inactive;118 schema
checks. First archive download failed; network-enabled retry passed. Default/cross-
target capture and foreign native execution are pending.

Frontier: checked SmallIndex::new/TryFrom still requires MAX, local Usize/core
conversion, scalar-error no-drop and Result helper transport. Scalar-error and
local-value-trait proof leaves exist unregistered and untested; do not treat them
as supported. Generated PatternID remains behind actual emitted callable/error
source ownership and ordered constructor callbacks. Preserve the committed
six-function selection, generated templates and frozen60 identities/denominators.

Next MAX dependency audit: `/tmp/rstyle-403-max-next.md`. Existing owned-Copy
autoborrow/helper snapshots fit MAX.as_usize; missing nominal-constant capture,
original initializer/dependency proof and complete product lowering remain.
Do not substitute the primitive u32::MAX contract for legacy core::i32::MAX,
or assume transparent layout proves evaluated fields. The full constructor
chain still needs Usize/core conversion and non-Copy Result transport.

Data3.96 connects original `<usize as Usize>::as_u64` through its debug
`u64::try_from(self).expect("usize overflowed u64")` body. Complete local trait
declaration/impl membership, requested/selected instances, original scalar
signature and ordinary source/build/body/summary proofs remain required. Exact
reviewed64-bit core authority proves always-Ok conversion with real TryFromIntError;
literal message evaluation is retained. Arbitrary/saved Results, narrowing,
effectful messages and escaping strings receive no new permission.

Fresh unchanged native no-default owner:109→110/590, one gain/zero losses.
Seven strict original/replay identity comparisons are Accepted score1; all21
selected original pairs finish. Std/no_std Cargo composition, semantic boundary
oracle, resealed message/selection attacks and3.95 downgrades pass. Four x86 Linux/
ARM macOS std/no_std captures strictly replay; foreign native execution remains
pending. Native public-API oracle:8,202 cases in each of four feature/debug contexts,
zero mismatches. All1,644 tests/five existing skips plus doctests, fmt/Clippy, style0/5538
and patterns0/15 unchanged ignores pass. Frozen `run-lz1z4duc` preserves all60
identities/denominators/outcomes at24 admitted/29 excluded/7 inactive;118 document
checks. Routine consistency stays XXH3; SHA remains for reviewed source pins and
historical claims, with no serialized lookup key. Method-shape filtering precedes call-inventory scans. Composition
test1.33s excludes compilation; this is not a controlled speedup claim.
Evidence:`/tmp/rstyle-404-{native-delta,native-strict,cross-results,frozen-comparison}`.

Next connected blockers: MAX typed value/initializer/dependency proof; named-scalar
non-Copy error no-drop receipt; exact core TryFrom/associated Error selection;
Result-aware original owner closure and fresh whole-result helper transport.
Existing scalar-error leaf remains unregistered. Generated PatternID additionally
needs emitted callable/error ownership and ordered constructor callbacks.
Readiness/write-set audits:`/tmp/rstyle-405-{max-ready,scalar-error-ready}.md`.

Data3.97 foundation (checked-index error chain): fresh named direct-scalar errors
in a captured Result return receive an independent original declaration/full-source
no-drop receipt, without Copy permission. Standalone owned constructors retain
their existing restrictions. `Result<CopyScalarProduct, NamedScalarError>` supports fresh
construction, explicit early error returns and whole helper transport. Storage,
owned Result parameters/reuse, nested ownership and payload binding remain closed.
The existing bounded source receipt store now distinguishes ordinary/scalar-error
proof modes while retaining source/cfg/profile/verification-policy separation;
routine checks remain XXH3, with no serialized lookup key or new cache substrate.

Fresh unchanged native owner:110→110/590, zero gains/losses and identical function
identities; capture/assessment complete, full-library matching explicitly
zero-budget incomplete. The Cargo constructor→forwarder composition, independent
boundary-value oracle, strict serialized verification and3.96 downgrade negatives
pass (0.567s final-suite sample excluding compilation). Natural checked construction
remains incomplete behind MAX value/source/dependency evidence and exact core
TryFrom/Error/source-owner linking. Source-first selection remains unchanged.
Evidence:`/tmp/rstyle-406-{composition.log,native-delta.json,tryfrom-ready.md}`.

Checkpoint:1,659 tests/five existing skips and doctests; fmt, warning-denied Clippy,
style0/5,559 and patterns0/15 unchanged ignores pass. Four x86 Linux/ARM macOS
std/no_std diagnostic Cargo contexts retain all three selected positives and
three negatives; strict original/replay verification/admission and complete
selected matching pass. Foreign native execution is not claimed. Frozen
`run-8c5r6fna` preserves all60 identities/denominators/outcomes at24/29/7 and118
document checks. Evidence:`/tmp/rstyle-406-{full-tests-final2.log,cross-results.json,
frozen-comparison.json}`. Next delivery must join MAX source/type/dependency
adapters and exact TryFrom/Error/source-owner receipts through the original
constructor chain; no separate nominal-constant coverage gain is claimed.

Data3.98 connects the original SmallIndex checked constructor chain: MAX has
independent typed initializer/source/core-dependency proof; exact local
TryFrom/associated Error selection feeds the ordinary helper pipeline; success
Copy and error no-drop receipts remain separate. Original Result source closure
checks relevant neighboring expansions and generated prelude shadows. The
[bounded contract](extraction-profiles/checked-index-construction-v1.md) excludes
general traits, generated callables and owned Result storage/reuse.

Baseline9dd9abc/Data3.97 → this checkpoint:110→112/590 in the unchanged native
no-default owner, exactly SmallIndex::new and TryFrom::try_from gained, zero losses;
all590 identities and23 source snapshots unchanged. Native default capture also
admits the chain (158/2582 overall; no same-revision default delta claimed).
Both contexts strictly verify/admit/replay nine originals, nine identity pairs
Accepted score1 and all36 selected original pairs. Full-library matching remains
explicitly zero-budget incomplete. Four x86 Linux/ARM macOS std/no_std diagnostic
contexts strictly replay eight positives and complete selected matching; foreign
native execution remains pending. Native original oracle passes8,202 public cases
per default/no-default × debug/release-branch context, zero mismatches.

All1,708 tests/five existing skips and doctests pass; fmt, warning-denied Clippy,
style0/5,738 and patterns0/15 unchanged ignores pass. Frozen run-bjl647cw preserves
all60 identities/outcomes/denominators/blocker combinations at24/29/7 with118
schema checks. Final composition sample2.531s excludes compilation; no controlled
speedup claim. Routine consistency uses XXH3/direct typed comparison; no runtime
SHA, serialized lookup key or new cache substrate was introduced. Shared checks
replace nine duplicated patterns. Evidence:`/tmp/rstyle-408-{native-final-delta.json,
native-final-strict.log,default-strict.log,cross-results.json,oracle-final.log,
frozen-comparison.json,full-tests-final2.log,style-final3.json,patterns-final3.log}`.

Next boundary: whole original generated PatternID::new. Known DAG: exact local
index_type_impls definition/invocation/emitted member and node ownership → nested
Copy success/non-Copy tuple-error receipts → consuming constructor-only Result.map
and map_err → ordinary summaries/strict replay. Reuse immutable snapshots/XXH3,
FileSpan/ExpansionStep, macro_scope and exhaustive cleanup interpretation; token
substitution alone cannot certify callable hygiene or source correspondence.
Next assignments: driver owns a generated-observation leaf; source-proof worker
owns a separate verifier leaf; Result worker owns nested transport/constructor
semantics; coordinator owns shared schemas, source/type routing and integration.
Freeze raw provenance and private proof interfaces before production edits.
Readiness requires exact emitted/template identities, complete active rosters,
independent branch/once-only/error-value expectations and resealed provenance,
Drop/derive, constructor-selection and duplicated-consumption attacks.
PatternID::must/Match::must retain expect/formatting/panic and instance blockers;
SmallIndexError::attempted still needs borrowed non-Copy owner source proof.
P01–P22, native foreign CI, detector quality and rewrite safety remain separate.

Generated-callable preparation (Data3.98 unchanged): opt-in raw capture retains
one unit table, exact expansion/member/node identities and hygiene, plus bounded
unary tuple-constructor instance observations. Original lexical macro verification
checks exact source anchors and all-ident substitution; it grants no callable or
cleanup permission. Immutable XXH3 snapshots and existing identity/HIR helpers are
reused; no SHA or serialized lookup keys were added.

Unchanged native no-default selection remains112/590, gain0/loss0 with identical
identities; capture/assessment complete, full-library matching explicitly incomplete.
Strict owning-unit replay and original PatternID source probe pass (four bindings,
22 emitted top-level items). The raw table retains two unsupported iterator
observations; global completeness is false. Whole PatternID::new remains excluded.
Evidence:`/tmp/rstyle-409-{native-delta.json,native-observations.json,original-source2.log}`.

Next connected activation needs an exact selected impl/member/node/hygiene join,
nested Copy success and independently no-drop error receipts, then a fresh-only
three-Result constructor-map witness. Each unchanged branch must rewrap its payload
in the new Result type; final-return error authority cannot certify intermediate
errors. Preserve once-only evaluation/consumption and the original error value.

Checkpoint:1,721 tests/five existing skips and doctests pass; fmt and warning-denied
Clippy pass; style0/5,792 and patterns0/15 unchanged ignores. The new diagnostic
Cargo regression passes in0.390s excluding compilation; no controlled speedup
claim. Evidence:`/tmp/rstyle-409-{source-final.log,full-tests-final2.log,
clippy-final3.log,style-final2.json,patterns-final2.log}`.

Data3.99 validation: all1,773 tests/five existing skips and doctests pass; fmt,
warning-denied Clippy, style0/6,025 and patterns0/15 unchanged ignores pass.
The constructor composition takes1.564s with prebuilt driver; native public oracle
passes8,202 cases in each default/no-default × debug/release-branch context,
zero mismatches. Ten original/replay identity comparisons per target/feature
context are Accepted score1; all45 original pairs complete independently.
Full-library matching remains explicitly zero-budget incomplete.

Source routing now checks node/tail shape before transport proof, removing an
eager whole-HIR scan from unrelated source lookups. No new SHA computation,
serialized lookup key or permissive cache was added. Routine consistency uses
cached XXH3; historical identities and reviewed source pins retain their meaning.
Native no-default capture+strict takes35.219s; default291.366s (single samples,
not a controlled speedup). Cross-target no-default35.580–35.591s/default287.061–
288.757s. Evidence:`/tmp/rstyle-413-{natural-results.json,oracle.log,tests.log,
clippy.log,style.json,patterns.log}`.

Frozen run-dzwel7hd preserves all60 identities, denominators, semantic records
and blocker combinations against run-bjl647cw:24 admitted/29 excluded/7 inactive,
gain0/loss0,118 schema checks. This reproduction does not replace strict semantic
replay. Evidence:`/tmp/rstyle-413-{frozen.log,frozen-comparison.json}`.

Next source-selected frontier (all shared interfaces remain lead-owned):

| Assignment | Readiness and complete target |
| --- | --- |
| Namespace/source proof | Frozen time0.3.47 four-error-conversion family needs exact foreign anonymous-trait/generated export footprints, local globs and multi-arm source/cleanup closure. Existing raw captures are diagnostics, not admission. Prove these dependencies before payload activation; do not stack another observation-only delivery. |
| Static-reference ownership | Preserve every ComponentRange static name pointer/length/content and conditional flag. Independently prove complete Copy/static-reference and no-drop closure, including neighboring generated items. Depends on namespace/source proof. |
| Payload/error flow | Complete two-variant TryFromParsed construction and once-only guarded inverse match, exact payload component origins and nominal DifferentVariant Result error. Saved/reused owned Results and general matches remain separate. Depends on the agreed ownership/type boundary. |
| Nominal conversion summaries | Extend exact selected From/TryFrom source/associated Error/helper evidence to nominal endpoints. Deliver all four frozen originals with independent public conversion/private-constructor expectations and strict selected matching. Depends on the preceding receipts. |

UnicodeWordBoundaryError::check/LookSet::available and the ryu loop chain are
already delivered controls. StateID::new has a separately audited preceding-
expansion lookup blocker; removing it alone unlocks no audited higher caller.
Its whole forwarding callers also need generated-error ownership and Result
summary transport. PatternID::must/Match::must still require their distinct
panic/formatting and instance proofs. Keep these limitations separate from the
new constructor support; broad P01–P22 completion remains unproven.

## Post-3.99 performance checkpoint

Strict definition/CFG correspondence now uses borrowed indexes, preserving first
matches and subsequent complete validation instead of repeated full scans.
SourceAnalysis retains its source context during frontend checks; final checks
still run on every request. Result-map request decoding avoids a recursive JSON
clone. No new SHA computation, serialized lookup key or admission capability.

Warmup plus three alternating baseline/current samples on unchanged fixtures:
constructor composition 1.466–1.511s → 1.467–1.499s; six-context stored-slice
composition 9.288–9.353s → 9.393–9.465s. Peak RSS remains approximately120/136MiB.
These changes do not demonstrate a speedup on these fixtures; the slice sample
is modestly slower. Separate stage observations place3.36s in six Cargo captures;
inclusive verification stages must not be summed. Evidence:
`/tmp/rstyle-414-{alternating.jsonl,map-profile.log,slices-profile.log}`.

Fresh original regex-automata no-default capture retains113/590 admissions,
all source/build identities and exclusion reasons: gain0/loss0. All10 selected
originals strictly replay; all45 selected original pairs evaluate completely.
Full-library matching remains explicitly budget-incomplete. Evidence:
`/tmp/rstyle-414-{natural-results.json,native-nostd-strict.log}`.
Frozen run-pzjme0ax retains all60 identities, denominators, semantic records and
blocker combinations against run-dzwel7hd:24 admitted/29 excluded/7 inactive,
gain0/loss0,118 schema checks; this does not replace strict semantic replay.
Evidence:`/tmp/rstyle-414-{frozen.log,frozen-comparison.json}`.

Integrated gates:1779 tests pass/5 existing skips, doctests, fmt and warning-denied
Clippy pass; rstyle0/6027 functions, patterns0 with15 unchanged ignores. Logs:
`/tmp/rstyle-414-{tests.log,clippy.log,style.json,patterns.log,fmt.log}`.
The four time error-conversion originals remain the next connected target;
their foreign namespace/static ownership/payload/conversion blockers persist.

### Data3.100: checked anonymous dependency namespaces

From4c32731, connect unit-local typed namespace capture, checked direct Cargo
artifact ownership and independent producer/consumer source receipts. The exact
unattributed direct glob can discharge lexical ambiguity in import and cleanup
checks; every other import/item remains checked. Generated/block-local/grouped
uses cannot borrow its receipt. Source caches retain owning evidence and separate
namespace capability/verification contexts. Routine checks use cached XXH3;
namespace duplicate detection now uses typed keys rather than JSON serialization.
[Bounded contract](extraction-profiles/anonymous-dependency-namespaces-v1.md).

Development constructor/getter/roundtrip-helper composition admits, matches and
strictly replays; unselected owners and neighboring/grouped unknown globs reject.
Independent source adversaries cover root/consumer attributes and resealed
spans/routes/identities. Native default-parsing capture of unchanged time0.3.47
plus num-conv0.2.0 completes1,363 definitions (time1,288, num-conv74, probe1).
Strict replay preserves all outcomes/reasons; all10 original anonymous prelude
footprints prove. The four selected error conversions remain excluded. A natural
admission delta is unavailable: preceding time diagnostics were capture-only and
incomplete. Pair matching is explicitly budget-incomplete in this large roster.
Evidence: `target/time-error-conversions-416/native-default-namespace/` and
`/tmp/rstyle-416-{time-assessment.json,original-namespace-proofs.json}`.

Remaining connected blockers: source-spelled static-string/complete Copy value
ownership, foreign generated time_core::convert and remaining local globs/macros,
guarded nominal payload origins, then exact conversion-member summaries. The
namespace prerequisite is integrated; do not stack further observation-only
increments or claim the natural family is delivered. Next source work must prove
the actual generated export footprint, while value work begins with the existing
Copy-component diagnostic rather than a general destructor model.

Frozen run-0a45q0bd preserves all60 identities, denominators, semantic records,
reasons and blocker combinations:24 admitted/29 excluded/7 inactive, gain0/loss0,
118 schema checks. All1,807 tests/5 existing skips plus doctests pass; fmt and
warning-denied locked Clippy pass, style0/6,113 and patterns0/15 unchanged ignores.
Suite observations: constructor composition1.442s, six-context stored slices9.384s;
no speedup claim. Logs: `/tmp/rstyle-416-{tests2.log,clippy-final3.log,style-final.json,patterns.log,frozen.log,frozen-comparison.json}`.

### Data3.101: supplied static-string Copy products

Connect the existing Copy-component source diagnostic to exact private value and
cleanup receipts, aggregate origins and ordinary helper summaries. No new lowering,
origin vocabulary, SHA computation or serialized lookup key. Original fields and
direct string formals must prove explicit immutable static lifetimes; unchecked
aggregate string formals remain excluded. Neighboring source and historical
policies retain their barriers. [Contract](extraction-profiles/static-string-copy-products-v1.md).

Development constructors/callers, saved-reference rebinding and two selected
nominal declarations admit, strictly replay and match. Native oracle checks
distinct equal-content backing, pointer/length preservation, flags and no drop.
Lifetime/Copy/unknown-expansion/origin/policy adversaries reject. Fresh unchanged
time/num-conv capture preserves all1,363 original identities and all outcomes:
6 admitted, gain0/loss0;25 reason lists advance to source-closure blockers. The
two selected ComponentRange constructors reach the unresolved convert glob;
nominal enum/Result payload/member work remains. Large-roster matching is still
explicitly budget-incomplete. Evidence: `/tmp/rstyle-417-time-{current.json,comparison.json}`.

Reassessment after two prerequisites without natural gains: prioritize the actual
time_core::convert source chain before adding more value features. Its compiler
footprint contains8 Struct and8 distinct Ctor exports, plus time's original public
reexport; select time-core explicitly, prove the complete original expansion
footprint and join the exact local reexport. This receipt must discharge only
proved namespace ambiguity; remaining local globs/macros and cleanup still prove.
Do not treat these observations as source authority or another delivered family.

Checkpoint: all1,811 tests/5 existing skips plus doctests pass; fmt and locked
warning-denied Clippy pass, style0/6,122 and patterns0/15 unchanged ignores.
The previously failing four borrowed-owner regressions pass. Static composition
0.619s and ownership/history controls0.855s are suite observations, not speedup
claims. Current strict replay preserves all1,363 natural identities, outcomes and
reasons with complete capture/assessment; large-roster matching remains incomplete.
Fresh frozen run-b2ehei6f preserves all60 identities, denominators, semantic records
and blocker combinations:24 admitted/29 excluded/7 inactive, gain0/loss0;
118 schema checks pass. Logs: `/tmp/rstyle-417-{tests-final.log,clippy-final4.log,style-final.json,patterns-final.log,frozen-final.log,frozen-comparison.json,time-replay-comparison.json}`.


### Data3.102: exact Copy error constructor sites

Reassessment replaces the constructor's coarse whole-unit cleanup traversal with
separate target and use-site receipts. Actual time0.3.47 ComponentRange has two
fields (static str/bool); the old i64 range-field shape is not this selection.
Exact positive Copy excludes target Drop in a complete successful compilation;
original declaration, static lifetime, full constructor source/HIR, literal flag,
formal identity, active attributes and inherent/declaration namespace all prove
independently. Target cleanup never substitutes for global namespace closure.
Out-of-module impls, unknown neighbors/callers and historical policies remain
closed. No new SHA, serialized lookup key, scoring change or ignore.
[Contract](extraction-profiles/copy-constructor-sites-v1.md).

Explicitly supplying time-core first produced three common-original gains:
Month::length and time's days_in_month/days_in_year_month. Common original
selection1,363 advanced6→9, zero losses; added44 dependency definitions are a
separate denominator (11 admitted). A Gregorian table/modulo oracle checks19,308
cases/57,924 API comparisons with zero mismatches. The constructor increment then
gains exactly ComponentRange::{unconditional,conditional} on the unchanged expanded
selection1,407:20→22 admitted, zero losses. Strict replay preserves all identities,
outcomes/reasons, completes capture/assessment and all231 pairs. Public/private
error-flow witnesses pass7/15 cases per default/no-default parsing context,
including exact backing, lengths, flags and negative tags. New native Cargo tests
cover unknown siblings with Drop, out-of-module impls, old policies, forged Copy,
trait/nominal identity, literals, local selectors and hidden HIR work.

The generated namespace preparation is retained as test-only source/export/roster
proof. Independent original capture audit consumes1,010 emitted definitions,
33 expansions and16 exports; focused adversaries pass. It grants no production
namespace permission. The unactivated full-module wrapper still encounters
another source attribute restriction. Do not rerun the bounded roster per import
when connecting it later; retain one immutable producer receipt and bind exact
consumer sites separately. Evidence: `/tmp/rstyle-418-original-roster-final.log`.

Next connected target: original From<ComponentRange> for TryFromParsed and inverse
TryFrom. The source audit below corrects the earlier caller assumption. All known prerequisites remain explicit:
static-reference mixed-enum vocabulary; exact nested positive Copy and nominal
unit DifferentVariant cleanup; variant construction, saved tag, guarded whole
payload binding and wildcard complement; Result<ComponentRange,DifferentVariant>;
tag-specific origins and independent document/helper component reconstruction;
selected core trait/member/associated Error evidence and caller linking. Current
origin handling has no MixedEnum selector: enabling syntax first would silently
lose reference dependencies. No blanket enum/Result activation is justified.

Coordinated next write assignments: parent owns TypeTree/token/origin/producer and
shared summary interfaces; source lane owns new copy_conversion_sites receipts;
value lane owns a bounded copy_tagged_values subtree with independent semantic
expectations; callable lane audits exact selected trait-member/caller linking;
namespace lane connects the retained proof only if this target actually needs it.
Freeze the guarded component interface before production edits; no new observation
catalogue or generic solver is a prerequisite for these exact originals.

Checkpoint: all1,830 tests/5 existing skips plus doctests pass; fmt and locked
warning-denied Clippy pass, style0/6,147 and patterns0/15 unchanged ignores.
Final strict replay preserves all1,407 identities/outcomes/reasons and completes
all231 pairs. Frozen run-y_jf65v0 vs run-b2ehei6f preserves all60 identities,
denominators, semantic records and blocker combinations:24 admitted/29 excluded/
7 inactive, gain0/loss0. Evidence:
`/tmp/rstyle-418-{natural-final-comparison.json,original-oracle.log,tests-final2.log,clippy-final.log,style-final2.json,patterns-final2.log,frozen-comparison.json}`.

### Tagged Copy conversion prerequisite (Data3.102; no activation)

The frozen [time error selection](../eval/coverage-development/time-error-conversions/selection.json)
now retains complete static-string mixed-enum and nominal-unit-error Result
observations. Reference containment inspects both Result branches and every enum
payload independently of admission. Unsupported guarded paths reject; they cannot
become an empty origin roster or bypass admission inside a borrowed owner.

Both unchanged original conversion methods pass separate source/positive-Copy and
exact compiler-selected From/TryFrom/member/associated-Error proofs in native
default+parsing and no-default+parsing contexts. Scoped owner normalization retains
actual DifferentVariant rather than replacing it with unit. Existing conversion
capture schema, reviewed core pins and structural identity keys are reused.
Selected reexports now resolve through proven loaded child modules; direct local
type names do not traverse a parent's module/value namesake. Imported ambiguity,
unknown neighbors and namespace/effect authority remain separate. New routine
fixtures use XXH3; no SHA or serialized lookup keys were added.

The unchanged default selection remains22/1,407 admitted, gain0/loss0, with all917
owned source records equal. Strict replay preserves all1,407 identities/outcomes/
reasons and completes231 pairs. Frozen run-rgsv2ohk preserves all60 identities,
denominators, semantic records and blocker combinations:24/29/7. Runtime oracle:
7 public original and15 private diagnostic cases per feature context, zero
mismatches. All1,857 tests/five existing skips plus doctests,23 inventory bookkeeping tests,
118 schema-checked documents, fmt, locked warning-denied Clippy, style0/6,211 and
patterns0/15 unchanged ignores
pass. The two focused Cargo tests together take0.39s with the prebuilt driver.
Evidence: `/tmp/rstyle-419-{natural-final-comparison.json,natural-replay-comparison.json,original-source-final.log,original-no-default-source.log,original-oracle.log,frozen-comparison.json,tests-final2.log,clippy-final5.log,style-final7.json,patterns-final3.log}`.

Next delivery must connect saved tag/value flow, guarded whole-product payload
binding, exact Result alternatives, tag-specific reference/component routes and
ordinary caller summaries through producer, independent document verification,
policy and adversarial composition tests. Product field indices or flattened
origin unions cannot encode this contract. Reuse existing enum selectors and CFG
guard checks; do not stack another observation catalogue. No original production
caller of the inverse exists in this time version. The nearest production forward
caller, utc_offset_try_from_parsed, still needs const-instance selection, Parsed
getters/Option tuples, and_then/unwrap_or, exact Into-to-From selection and map_err
composition. The two methods alone do not unlock that parser. Broad P01–P22 work
remains active; this prerequisite grants no new admission capability.

### Guarded conversion callable composition (Data3.104)

Exact original local From/TryFrom members now publish typed guarded summaries
only after complete graph and component-origin verification. Explicit same-unit
calls preserve selected member/build/source authority. A bounded wrapper accepts
one immutable formal, saves it once, invokes the conversion and immediately returns
the whole result. Callee alternatives remain scoped beneath that caller call;
callee HIR guards never become caller CFG guards. Wrappers do not yet publish
callable summaries, project payloads or accept arbitrary conversion expressions.

The real-Cargo composition test covers four admitted forward/inverse wrappers,
an independently compiled runtime oracle, strict import, complete matching,
17 graph mutations and five resealed summary mutations under strict and generated
verification. Its focused prebuilt-driver run took3.68s. Routine identities remain
cached XXH3; no new SHA or serialized lookup keys were introduced.

Fresh unchanged time/time-core/num-conv selection:24/1,407 admitted, gain0/loss0;
all remaining outcomes/reasons and all276 pair results match Data3.103. Strict
replay agrees completely. Evidence: `/tmp/rstyle-424-{natural.json,strict.json,delta.json,composition6.log}`.
This foundation does not complete an original caller family. The nearest time
parser uses function-item Into/map_err composition plus Parsed/deranged/Option
dependencies. Source audit also shows its checked error branch is unreachable
under the valid Parsed range invariants; it is a poor practical error-flow target.

Reassessed next target: unchanged time0.3.47 ExactlyNDigits::<1/2/3>::parse,
default+parsing and no-default+parsing. Independently expected: consume exactly
N leading ASCII digits, return their decimal value and input suffix; otherwise
None. Check all bytes, all valid combinations, short inputs, nondigit positions,
non-ASCII bytes, suffix pointer/length and arithmetic bounds. These are distinct
concrete method definitions already captured, not generic function instances.
No full parse_subsecond caller claim follows from these leaves.

Readiness/write boundaries for the next coordinated increment:

- Capture lane: complete slice/@range/rest HIR and binder observations in a
  dedicated driver leaf; lead owns shared HIR/source registration. Existing
  pat_unsupported omits children and bindings, so fresh capture is mandatory.
- Product lane: exact ParsedItem<u8/u16> declaration/substitution/field receipts
  and independently proved no-drop. ParsedItem is non-Copy; generic T can drop.
  Reuse constructor instance evidence, not displayed type strings or Copy fiction.
- Pattern/origin lane: length-before-read prefix tests and exact suffix region,
  including an empty one-past suffix. Mismatch still reads selector bytes. Original
  body dereferences remain separate guarded reads. Do not synthesize source indices.
- Lead: concrete impl/source containment, Option success/failure composition,
  policy/admission/matching integration and shared types/origins. Keep neighboring
  derives, methods, macros and unsupported contexts visible and separately checked.

Readiness evidence: `/tmp/rstyle-424-digits-source-audit.md` and
`/tmp/rstyle-424-digits-prefix-origins-audit.md`. Original functions863–865 are
currently excluded; no admission gain is predicted. Open seams are concrete product
descriptor/source validation, no-drop authority and guarded Option component
encoding. Freeze these interfaces before parallel production edits. Broad P01–P22
work and cross-target/native validation remain active.

Integrated checkpoint:1,906 tests/five existing skips and doctests pass, including
the complete composition test at3.80s. Formatting, locked warning-denied Clippy,
style0/6,378 and patterns0/15 unchanged ignores pass. Frozen run-t6jmzf89 checks118
documents and preserves all60 identities, denominators, outcomes/reasons and
blocker combinations against run-rgsv2ohk:24 admitted/29 excluded/7 inactive,
gain0/loss0. Evidence: `/tmp/rstyle-424-{tests-final2.log,clippy-final4.log,style-final2.json,patterns-final2.log,frozen.log,frozen-comparison.json}`.


### Digit-prefix capture and source proof (Data3.105; admission closed)

Original time0.3.47 ExactlyNDigits::<1/2/3>::parse now retains complete slice,
@binding, byte-range and real rest HIR plus exact lexical binder inventories.
Distinct ParsedItem<u8/u16> observations preserve the original generic source;
the origin collector now retains that declaration instead of dropping it.
Independent source proofs and adversaries check spans, modes, normalized types,
child identities, endpoints and duplicate bindings. Shared byte-view predicates
replace two exact clones. Routine identities remain cached XXH3.

Validation:1,931 tests/five existing skips and doctests pass; fmt and locked
warning-denied Clippy pass; rstyle0/6,446, patterns0 with15 unchanged ignores.
Original native oracle:229,554 checks per default/parsing and no-default/parsing
context, zero mismatches. Fresh default/parsing capture and strict replay retain
all1,407 identities/outcomes,24 admissions, gain0/loss0 and identical complete276
pair results versus Data3.104. Composition regression:3.72s. Evidence:
`/tmp/rstyle-425-{digit-oracle.json,digits-capture2.json,strict.json,tests-final.log,clippy-final2.log,style-final.json,patterns-final.log}`.
The historical frozen60 was not rerun at this prerequisite checkpoint.

Next connected work remains the same three original parsers: exact
instance-specific source/substitution/no-drop receipt; real binary constructor
identity (current normalized expr_struct omits its callee); ordered length/digit
selector reads, guarded body dereferences and saved-input suffix regions; then
Option product construction, independent graph replay and strict matching in
both original feature contexts. Proof and semantic-model leaves are prepared but
unregistered/unvalidated; they are not delivered support. No nominal-only cleanup
permission or fresh-product slot READ is justified. Full parse_subsecond and
broader P01–P22 remain active.


### Connected digit-prefix values (Data3.109)

The root roadmap records the bounded contract and unchanged selections. Default
Cargo capture/strict replay gains the three original ExactlyNDigits parsers and
two audited ComponentRange getters:24→29/1,506, zero losses,276 old pair results
preserved and406 current pairs complete. The no-default selection stays22/1,406,
zero gains/losses and231 old pairs preserved. Its num-conv0.2.2 prelude adds Widen
and lint metadata to the two-export proof; downstream source closure beyond that
barrier remains unproved. Do not substitute the default lockfile to report gains.
The original native oracle retains229,554 checks per context, zero mismatches.

Next frontier: independently prove the complete newer anonymous-trait namespace,
then recheck all no-default blockers. Callable summaries for Option<ParsedItem>
remain a separate value/component/continuation contract; neither these admitted
leaves nor the source-only trait receipts establish parse_subsecond support.
N=4..9 and their callbacks/library operations remain excluded. No new runtime SHA
operations, ignores, suppressions, scoring changes or frozen selection edits.


Validation:2,088 workspace tests pass/five existing skips plus doctests; fmt,
warning-denied Clippy,style0/6,923 and patterns0/15 unchanged ignores pass.
Composition/adversarial test:8.31s on this runner; no controlled speedup claim.
Fresh frozen run-p29c1auq verifies118 documents and preserves all60 identities,
denominators, outcomes/reasons and blocker combinations:24 admitted/29 excluded/
7 inactive, gain0/loss0 against run-t6jmzf89. Foreign native CI remains unverified.
Evidence: `/tmp/rstyle-432-{nextest2.log,clippy5.log,fmt-final.log,style4.json,patterns3.log,frozen-comparison.json,default-final-strict-comparison.json,no-default-final-strict-comparison.json}`.
Original default run: `target/rstyle/clones/run-dFHSan`; no-default strict replay:
`target/rstyle/clones/run-jOLgpB`. All capture/assessment and natural selected-pair
completeness checks pass; frozen matching remains intentionally budget-limited.


Data3.110 / dependency source increment (fb4038a baseline): original
num-conv0.2.2 Extend/Truncate/Widen export closure now checks all three identities,
direct routes, complete module footprint and original attribute-free compiler
spans. Only independently validated deprecation metadata gains permission; no
trait bodies/effects are certified. Reuses checked owning units and cached XXH3
source identities; selects the bounded shape before parsing rather than retrying
the full legacy proof. No new ignores, suppressions, thresholds or source omissions.
Fresh no-default run-fRDbW5 preserves1,406 identities,22 admissions and231 pairs;
the three digit parsers now reach the next source barrier. Source audit identifies
powerfmt0.2.1's changed alternate->with_alternate arm and docsrs root attributes;
0.2.0's complete source receipt deliberately rejects it. Deranged0.5.9 is another
changed dependency whose downstream closure remains unknown. Complete that
source chain before claiming no-default digit support or advancing caller summaries.

Validation:2,091 workspace tests pass/five existing skips plus doctests; fmt,
warning-denied Clippy,style0/6,929 and patterns0/15 unchanged ignores pass.
New Cargo composition/strict matching test:0.54s locally, no speedup claim.
No-default strict run-fRDbW5 retains all231 pairs. Default fresh run-TI3XOT
retains29/1,506 admissions and all406 pairs, gain0/loss0. Replaying the older
3.109 default bundle under3.110 rejected hint::cold_path operation provenance;
checks were preserved and the identical locked context recaptured. The frozen
roster and foreign native CI were not rerun for this source-only increment.
Evidence: `/tmp/rstyle-433-{unit2.log,cargo-test.log,nextest.log,clippy2.log,fmt.log,style-final.json,patterns.log,no-default-strict-comparison.json,default-fresh-strict-comparison.json}`.


Data3.111 / corrected macro source (2480d84 baseline): independently reviewed
powerfmt0.2.1 changes alternate->with_alternate, docsrs metadata and inline
annotations. New capability checks the exact paired root/module bytes, original
macro/export spans, all14 arms, active cfg and the same compiled dependency edge.
A mixed0.2.0/0.2.1 pair is rejected even after consistent rechecksums. Independent
expression expectations retain each value/option; legacy review settings cannot
accept the new source. No callable/effect/purity contract or runtime SHA is added.
Original no-default run-9kd3tB and strict replay preserve all1,406 identities and
admit27 (22 baseline), gaining the three selected digit parsers and two audited
ComponentRange getters, loss0. All231 old pair records persist; all351 current
pairs complete. This completes the selected leaves in both feature contexts;
Deranged0.5.9 introduces no further blocker for this selection. parse_subsecond
remains excluded: N4..9 array-map/iterator callbacks, Option/product helper and
residual/component flow, map conversion callbacks, and the OneOrMore loop branch
are still a connected dependency chain. No supported-prefix caller claim follows.

Validation:2,094 workspace tests pass/five existing skips plus doctests; fmt,
warning-denied Clippy,style0/6,929 and patterns0/15 unchanged ignores pass.
Default run-NA2mz9 and strict replay retain29/1,506 admissions and406 unchanged
pairs, gain0/loss0. Original native oracle:229,554 checks/context, zero mismatches.
Fresh frozen run-0w329uds verifies118 documents and preserves all60 source
identities, denominators, outcomes/reasons and blocker combinations:24 admitted,
29 excluded,7 inactive; only producer revision and transient capture paths/IDs differ.
Foreign native CI remains unverified. Logs: `/tmp/rstyle-434-{nextest.log,clippy.log,fmt.log,style.json,patterns.log,oracle.json,frozen-comparison.json,no-default-strict-comparison.json,default-strict-comparison.json}`.
Next bounded callers: original rfc::iso8601::{hour,min}, both direct two-digit
forwarders in these same locked contexts. Prove complete Option/product return
transport, conditional prefix reads and exact concrete call linkage before source/
cleanup receipts permit them. Expect decimal0..99 and exact input[2..] on success;
short/nondigit input returns None. Preserve mismatched reads, suffix/component
origins, callee identity/build and hidden-drop negatives. General callbacks and
parse_subsecond remain outside this first forwarding contract.

Data3.112 / digit-parser callable summaries (a8197d9 baseline): complete original
parser graphs supply exact read occurrences and a guarded suffix component;
caller activation remains closed. The separate `time-digit-forwarders` selection
fixes hour/min source/build identities, complete known blockers and independent
0..99/suffix expectations before admission. Native oracle184,194 checks/context,
zero mismatches. Fresh unchanged selections: no-default27/1,406, default29/1,506;
zero gains/losses and all351/406 strict pair records unchanged.
Evidence: `/tmp/rstyle-435-{forwarder-oracle.json,no-default-comparison.json,default-comparison.json}`.
Next: caller-specific namespace/cleanup receipt, exact concrete inherent call
proof, then callee-scoped conditional reads and complete tagged return transport.
Final gates:2,095 tests pass/five existing skips plus doctests; fmt, warning-denied
Clippy, style0/6,935 and patterns0/15 unchanged ignores. Frozen run-4r7lfxke
retains all60 identities/outcomes/blockers:24 admitted/29 excluded/7 inactive,
118 checked documents. Strict linked replay constructs all three original
summaries in both contexts; diagnostic derivation observations are2–8ms each,
not a controlled throughput comparison. New composition test2.1s; existing8.3s.
Logs: `/tmp/rstyle-435-{nextest-final.log,clippy-final.log,fmt-final.log,style-final.json,patterns-final.log,frozen-comparison.json,summary-timings.json}`.
Foreign native CI remains unverified. Caller support and the broad goal remain open.

Data3.113 / original digit forwarders (74c5550 baseline): hour/min now retain
one saved formal, the complete guarded callee return and distinct conditional
read occurrences. Caller-specific namespace/no-drop ownership and literal const
selection are independently checked; builtin Option lookup consumes the existing
complete anonymous-prelude proof. No graph duplication, new hashes or driver
instance transport. Wrong widths/implementations, altered call/result scope,
missing/merged reads, flattened origins, extra work and historical policy fail closed.
Fresh strict no-default run-v0MNmH:27→29/1,406; default run-FtevPu:29→31/1,506.
Only hour/min gain admission; zero losses, all351/406 old pair records unchanged,
all406/465 current pairs complete and the original wrapper pair Accepted in both.
Native oracle184,194 checks/context, zero mismatches. Frozen run-3lqxqg29 checks118
Data documents; all60 identities, denominators, outcomes/reasons and blockers
remain unchanged:24 admitted/29 excluded/7 inactive. Foreign native CI is unverified.
Final gates:2,097 tests pass/five existing skips plus doctests; fmt, warning-denied
Clippy, style0/6,964 and patterns0/15 unchanged ignores. New composition test1.79s,
including semantic call-selection and document attacks; no controlled speed claim.
Evidence: `/tmp/rstyle-436-{landing-nextest.log,landing-clippy.log,landing-fmt.log,landing-style.log,landing-patterns.log,no-default-comparison.json,default-comparison.json,oracle.json,frozen-comparison.json}`.
Remaining frontier: transitive whole-result callable summaries and guarded payload
consumption; week/day/dayo also require NonZero and verified callback composition.
parse_subsecond additionally needs N4..9, residuals/conversions and its loop branch.
These are complete caller chains to audit before activation; the broad goal remains open.

Data3.114 / checked native index decoding (97d500e working-tree integration):
original regex-automata0.4.18 SmallIndex::from_ne_bytes → PatternID::from_ne_bytes
now composes with the selected local U32::as_usize helper. Separate array-Result,
u32→usize and u32→u64 gates reuse existing ownership/source/summary authority;
no new runtime hashing, graph copying or serialized lookup. Historical policies
remain closed; malformed source, implementations, operands, graph and policy reject.
Retained native no-default captures run-FjQeCg → run-zlTUUT preserve590 identities/
build contexts and gain exactly those three:113→116, zero losses. Baseline producer
74c5550+dirty/policy3.113 is a retained executable, not a clean97d500e rebuild.
Strict original/replay:three Accepted score1; all three distinct-original pairs
complete and Rejected as predeclared (WL or size gate). Full-library matching
remains budget-incomplete. Native oracle270352 inputs/API/context, all four
feature/debug controls, zero mismatches; foreign native CI remains unverified.
Final gates:2,111 tests pass/five existing skips plus doctests; fmt, warning-denied
Clippy, style0/6,982 and patterns0/15 unchanged ignores. New composition1.77s,
a single observation. Frozen run-qgircimn checks118 Data documents and preserves
all60 identities/outcomes/reasons/blockers and denominators,24/29/7; older profiles
also unchanged. Evidence: `/tmp/rstyle-437-{comparison.json,strict-final.log,oracle.json,nextest-final.log,clippy-final.log,fmt-final.log,style.json,patterns.log,frozen-comparison.json}`.
Next shared source bottleneck: generated-source lookup treats an earlier local
item-macro invocation as unknown and poisons later textual lookup (StateID follows
PatternID). Prove the complete preceding expansion's namespace effects before
changing that restriction; sharing a macro template alone is insufficient.
Transitive result summaries and week/day/dayo's NonZero/callback/residual chain
remain separate higher-caller work. The broad coverage goal remains open.

Data3.115 / ordered local macro source (8e8e0de baseline): original StateID::new
and StateID::from_ne_bytes now join exact source ownership after PatternID.
Complete bounded predecessor interpretation proves enclosing textual scope only;
existing namespace/hygiene, full member ownership, no-drop and callee checks stay
mandatory. Unknown scope is never healed. No new hashes or serialized lookup keys.
Retained native run-zlTUUT→run-YrcTin preserves590 function/build identities apart
from producer/navigation and gains exactly those two:116→118, zero losses.
Four original/replay pairs Accepted score1; six distinct-original pairs complete
and Accepted as independently specified. Full-library matching budget-incomplete.
Original constructor oracle8202 inputs/context in all four feature/debug controls,
zero mismatches; retained Data3.114 decoder oracle includes StateID. Foreign native
CI unverified. Positive composition2.01s, negatives0.97s with prebuilt driver;
initial focused25.19s included nested build, not a controlled comparison.
Remaining frontier: StateID siblings have distinct source/interface blockers;
transitive parser summaries, guarded payload consumption and source-required
NonZero/callback/residual work remain open. The broad coverage goal is not complete.
Validation:2,118 tests validated (2,117 passed full run; spacing-only style failure
corrected and its test rerun passed), five existing skips, plus doctests. Fmt and
warning-denied Clippy pass; style0/6,985 and patterns0/15 unchanged ignores.
Frozen run-usp9j60b checks118 Data documents; all60 identities/outcomes/reasons,
denominators and blockers unchanged24/29/7; R0/expanded also unchanged.
Evidence: `/tmp/rstyle-438-{comparison.json,strict.log,oracle.json,nextest-final.log,
style-retest.log,fmt-final.log,clippy-final.log,style-final.json,patterns-final.log,
frozen-comparison.json}`. Documentation lands with this implementation.

Nonzero digit-parser preparation (b0da666 baseline; policy stays 3.115):
source-selected original time 0.3.47 week/day/dayo connect digit parsers to
NonZero payloads. Bounded function-item and empty-closure identities now retain
nested flat_map calls in a separate callback scope with the correct typeck owner.
Capturing closures/function pointers remain unsupported; no body, effect or
admission authority follows. No new hashing or lookup cache.

Independent native oracle: 259,952 checks in each default/no-default context;
lexical values 01..99 and 001..999, exact suffix pointers, zero/short/non-digit
rejection, explicit non-calendar controls. Fresh no-default capture
run-03B1Il→run-NZ2AGB preserves all 1,406 function/build identities and outcome
reasons: 29 admissions, zero gains/losses. Strict replay verifies all 29 and
completes all 406 pairs, identical to the retained 3.113 strict outcomes/reasons
and pair records. The selected trio still excludes.

Validation: all 2,119 tests pass, five existing skips, plus doctests; final test
helper refactor also passes its focused test. Fmt and warning-denied Clippy pass;
rstyle 0/6,990, patterns 0 candidates/15 unchanged ignores. Frozen run-u1um8uxj
checks 118 Data documents; all 60 identities/outcomes/reasons, denominators and
blockers unchanged (24/29/7); R0/expanded also unchanged.

Next connect the core NonZero type/value/no-drop contract, complete original
flat_map source/substitution proof and guarded Option/suffix transport before
another observation-only increment. This is evidence preparation, not family
support or broad-goal completion. Evidence: `/tmp/rstyle-439-{comparison.json,
observations.json,oracle.log,strict-comparison.json,nextest.log,refactor-test.log,
clippy-final.log,style-final.json,patterns-final.log,frozen-comparison.json}`.

Nonzero semantic contracts (371dfdc baseline; policy remains 3.115): exact native
core u8/u16 NonZero construction/no-drop, original same-unit flat_map body and
structural substitutions, and paired product shapes now have independent checks.
No aggregate Copy, lexical, callback-guard or admission authority follows.
Natural capture exposed and fixed raw NonZero field handling in derived neighbors;
those neighbors remain captured and excluded. All 1,406 identities/build contexts
persist: 29 admissions, zero gains/losses; 18 excluded diagnostic lists change.
Strict replay verifies all 29 and preserves all 406 pair records exactly.
Validation: 2,145 tests pass, five existing skips, doctests, fmt/Clippy, style
0/7,050, patterns 0/15 unchanged ignores; real Cargo composition 0.37s with the
prebuilt driver (one sample, no speedup claim). No new hashing or serialized keys.
Next deliver complete original caller source/cleanup and guarded Option/suffix/read
composition; week/day/dayo remain excluded. This foundation does not finish the
family or broad goal. Frozen inventory was not rerun at this inactive checkpoint.
Evidence: `/tmp/rstyle-440-{comparison.json,strict.json,nextest-final.log,
clippy-neighbor.log,style-final.json,patterns-last.json,neighbor-inventory.log}`.

Nonzero family activation (42974a5 baseline; Data3.116): original time week/day/dayo
now connect complete caller source/cleanup, exact original flat_map substitutions,
pinned NonZero semantics and guarded suffix/read transport. Native unchanged
selections gain only these three:29→32/1,406 without defaults and31→34/1,506 with
defaults, zero losses. Strict replay verifies all32/34 and completes496/561 pairs;
all406/465 historical pair records persist. Capture output directories and producer
revision normalize; original identities and build settings remain identical.
Independent original oracle259,952 checks/context, zero mismatches.

Matching boundary: week/day Accepted score1 as expected; both cross-width pairs
with dayo also score1, failing the source-based negative matching expectation.
Current graph labels deliberately ignore types/operators/literals/callee names.
No detector/scoring changes; no behavioral-equivalence or quality claim follows.
Full post-refactor suite2,148 tests/5 existing skips plus doctests passes. Fmt and
warning-denied Clippy pass; style0/7,124 and patterns0/15 unchanged ignores. Shared
source-receipt, representation and sequencing helpers remove new duplication;
no new hashing or serialized lookup keys. Positive Cargo composition2.30s with
prebuilt driver (one sample, no performance comparison).

Next frontier: transitive callable summaries and broader concrete payload/error
flow. Original parse_date is not unlocked: its complete chain also needs captured
FnMut state, mutable Parsed/ExtendedKind updates, aggregate destructuring, relevant
macro ownership, multi-error Result conversion and the remaining year/month/dayk
helpers. Audit a smaller complete caller before another activation; do not count
this leaf family as the whole ISO-8601 parser. Broader Rust coverage remains active.
Evidence: `/tmp/rstyle-441-{strict-comparison.json,default-strict-comparison.json,
oracle.json,nextest-integrated.log,pattern-clippy.log,style-final.json,
patterns-final.json}`.
Frozen run-cjypvid0 verifies118 Data documents; all60 original identities,
assessments, blockers and denominators match run-u1um8uxj at24/29/7. R0 and
expanded also remain identical. Comparison: `/tmp/rstyle-441-frozen-comparison.json`.

Unsigned conversion family (54d95f0 baseline; Data3.117): original regex-automata
SmallIndex TryFrom<u16/u32/u64/usize> connects exact input/source/selected impl,
associated Error, local U16/U64 helpers and pinned core widening/native conversion.
Native no-default roster118→123/590: three conversion and two helper gains, no
losses, unchanged original identities/build contexts. Strictly verify/admit all123
from the complete owner and serialized replay; all7,503 original comparisons
complete. Each selected function matches its replay at score1; the six cross-input
comparisons reject at unchanged thresholds. No behavioral-equivalence claim.

Independent original oracle270,438 public-API checks/context across default/
no-default and debug true/false, zero mismatches. Exact core pins captured for
Linux ARM/Linux x86/macOS ARM; foreign native CI and default/release extraction
are not claimed. Full suite2,154 passes/5 existing skips plus doctests; fmt and
warning-denied Clippy pass; style0/7,125, patterns0/15 unchanged ignores. The
complete Cargo fixture takes2.41s with prebuilt driver in the full run (single
sample, no performance comparison). No new runtime hashing/serialized lookup.
The conversion table shares production and document-policy selection.

Next frontier: generated PatternID/StateID TryFrom callers remain excluded.
They require independently verified trait-impl/associated Error expansion
ownership followed by complete Result map/map_err value/error transport; existing
inherent-constructor evidence does not cover them. Other error/payload, generic
and memory lanes remain separate. Broad coverage goal stays active.
[Source contract](../eval/coverage-development/unsigned-index-conversions/README.md).
Evidence: `/tmp/rstyle-442-{comparison.json,oracle.json,all-strict-summary.json,
selected-strict-summary.json,nextest.log,clippy-recheck.log,style-final.log,
patterns-final.log,pin-test-final.log}`.
Frozen run-luj4fm3q checks118 Data documents; all60 identities/assessments,
blockers and denominators match run-cjypvid0 at24/29/7. R0 and expanded also
match exactly. Final target-pin cardinality/identity tests pass3/3; fmt/Clippy
rechecked clean. Comparison: `/tmp/rstyle-442-frozen-comparison.json`.

Generated conversion source foundation (7cba124 baseline; Data3.117 unchanged):
all eight original PatternID/StateID unsigned TryFrom members pass exact source,
associated Error/projection, core identity, substitution and complete HIR-role
proof. Production admission remains closed. Unchanged native no-default roster:
123/590, no gains/losses; all identities and build contexts persist. Only the
eight wrappers change exclusion reasons after retaining the new observations.

The Cargo fixture checks both owners/four inputs and184 adversarial mutations;
raw capture also checks generic/lookalike/inactive and historical-profile cases.
Full suite2,157 passes/5 existing skips plus doctests; fmt/Clippy clean;
style0/7,141 and patterns0/15 unchanged ignores. New composition1.35s in the full
run (one sample, no speedup claim). Shared original/generated signature checking
removes duplicate proof logic; no new runtime hashing or serialized lookup keys.
Next connect exact third trait-call evidence and complete lexical/no-drop proof;
do not relax the existing two-map roster or certify neighboring impls by name.
[Contract](../eval/coverage-development/generated-index-conversions/README.md).
Evidence: `/tmp/rstyle-443-{comparison.json,source-tests.log,source-probe.log,
nextest.log,clippy.log,style.json,patterns.json,strict-comparison.json}`.
Strictly verify/admit all123 from the complete unit and serialized replay;
all246 imports and7,503 pair records match the previous checkpoint exactly.
Broad coverage goal remains active.
Frozen run-u89zcvsw verifies118 Data documents and matches run-luj4fm3q:
all60 identities/results/blockers/denominators unchanged at24/29/7; R0 and
expanded also match. `/tmp/rstyle-443-frozen-comparison.json` retains the comparison.


Data3.118 connects all eight original PatternID/StateID unsigned TryFrom
wrappers through the complete three-call chain and independently verified
SmallIndex implementation. Exact requested/selected identities, substitutions,
lexical ownership, Copy success and no-drop error remain required. Cheap raw
candidate indexing precedes owning-unit proof; no new hashes or serialized
lookup keys. Native no-default admissions123→131/590: eight gains, zero losses,
all source identities/build contexts unchanged.

Full suite2,158 passes/5 existing skips plus doctests; fmt/Clippy clean;
style0/7,156 and patterns0/15 unchanged ignores. Positive composition4.802s
(one sample),288 adversarial mutations plus selected-body corruption, historical
policy and mixed-owner negatives. Independent original oracle270,438 checks per
context across four feature/debug contexts, zero mismatches. All131 admissions
pass original/replay verification (262 imports); complete8,515 comparisons retain
all7,503 historical pair records exactly. All28 new-family pairs are structurally
Accepted at1, including cross-width inputs: no behavioral equivalence or rewrite
safety claim. Frozen run-lz6j5gtm matches run-u89zcvsw: all60 identities/results,
blockers/denominators at24/29/7,118 Data documents; R0/expanded unchanged.

Remaining frontier: audit complete transitive callers and their guarded payload
consumption before extending summaries. Imported generated payload aliases,
unresolved neighboring macros, generic instances and broader cleanup remain
closed; PatternID::must still needs expect/formatting/panic evidence. Do not
count these eight wrappers as supporting their higher callers. Broad goal active.
[Contract](../eval/coverage-development/generated-index-conversions/README.md).
Evidence: `/tmp/rstyle-444-{comparison.json,strict-comparison.json,oracle.json,
frozen-comparison.json,nextest.log,clippy.log,style.json,patterns.json}`.


Wire arithmetic prerequisite (Data3.118/145 capabilities unchanged): source-first
selection fixes original regex-automata add/mul → arithmetic_overflow, complete
10-variant/15-field error and both generated children. Cargo capture now retains
the complete payload tree/Result rather than losing it at the first named enum.
Existing policies explicitly reject these diagnostic shapes; no ownership, Copy,
cleanup, origin or callable authority follows. Hidden Drop/memory payloads and
18 omitted/reordered/changed-field attacks reject. Native no-default131/590,
gain0/loss0, identities/build contexts unchanged. Independent original oracle:
443,721 mathematical/error-variant/static-pointer checks in each of four contexts,
zero mismatches. New Cargo regression0.386s; full suite2,160 passes/5 skips plus
doctests, style0/7,159 and patterns0/15 unchanged ignores. No new hashing,
serialized lookup keys or whole-unit parsing/indexing.

Next: detach the existing generated declaration proof from current map-callable
ownership; resolve exact import/reexport chains to those checked declarations;
prove the entire error's recursive no-drop closure; then compose guarded
checked-Option payloads and constructor/helper Results. Current source failure
is the generated nominal import terminus, not an invalid alias. No additional
capture protocol is justified. HalfMatch::must additionally requires actual
Debug/formatting effects; ISO parse_date also needs stored mutable closures and
several missing parser families. The memory audit selects LineWrapper::wrap_blocks
as a later Result/post-state bridge, without claiming Encoder::encode support.
[Selection/contract](../eval/coverage-development/wire-arithmetic-errors/README.md).
Evidence: `/tmp/rstyle-445-{natural-shapes.json,comparison.json,oracle.json,nextest.log,
clippy-final.log,policy-final.log,capture-final.log,style-final.json,patterns-final.json}`.
Final fmt/Clippy and focused policy/Cargo rechecks pass after consolidating the
new guard into one tree traversal. All131 original/replay checks (262 imports)
and8,515 pair records match the prior checkpoint exactly. Frozen run-yvqmo2nk
matches run-lz6j5gtm: all60 identities/results/reasons/blockers/denominators,
24/29/7 and118 verified Data documents; R0/expanded unchanged. Comparisons:
`/tmp/rstyle-445-{strict-comparison.json,frozen-comparison.json}`. Broad goal active.

Generated error declaration proof (Data3.118 unchanged): ordinary callers now
verify exact generated tuple declarations without a generated callable receipt.
Declaration-only macros seed the existing unit roster from captured origins;
no new hashes, serialized keys or source-index pass. Callable owner checks remain
intact; cfg-inactive declarations, conflicting module/compiler identities and
incomplete/forged root or child rosters reject. All six declaration source proofs
pass on the three unchanged wire originals. This grants no derive, cleanup, Copy
or callable authority. Fresh no-default remains131/590, zero gains/losses, with
identical identities, build contexts, outcomes and reasons. Frozen60 remains
24/29/7 with all profiles and blocker combinations unchanged.

Two preparatory increments have produced no admissions. Source observation and
declaration ownership are now sufficient for this target: prioritize connecting
checked import termini, recursive whole-error no-drop and guarded Result/helper
composition before further diagnostic expansion. Broad goal remains active.
Validation:2,164 tests pass (the full run's spacing failure was corrected and
rechecked separately), plus doctests; final fmt/Clippy clean, style0/7,167 and
patterns0/15 unchanged ignores. All262 original/replay imports,131 self-comparisons
and8,515 original pair records match the prior checkpoint exactly. New Cargo
composition regression0.48s in the focused run. Native foreign-host CI is unrun.
Evidence: `/tmp/rstyle-446-{nextest.log,final-test.log,clippy-final.log,fmt.log,style.json,patterns.json,comparison.json,frozen-comparison.json,strict-comparison.json,fresh-probe.log}`.

Connected wire arithmetic errors (Data3.119/147 capabilities): complete ten-variant,
15-field/five-nominal no-drop closure and checked generated import termini compose
with static-field constructor summaries and guarded Option→Result arithmetic.
Native no-default131→138/590, zero losses; all source/build selections persist.
Four incidental constructors fit the same source-audited contract. Strict276
original/replay imports,138 self-comparisons and9,453 cross-pairs complete;
all131 prior verification records/8,515 pair records are identical. Structural
add/mul acceptance at1 fails semantic separation; no equivalence/rewrite claim.
Frozen run-5d5p51ww preserves all60 identities/results/reasons/blockers/denominators,
24/29/7 and118 Data documents; R0/expanded unchanged. Full2,177 tests/5 existing
skips plus doctests pass; final routing refactor17 focused tests/doctests pass.
Fmt/Clippy clean, style0/7,256, patterns0/15 unchanged ignores. Fifteen resealed
origin/guard attacks and direct/reexported hidden Drop negatives reject.

Default-feature capture is incomplete: existing concrete-observation depth/entry
budget aborts at sparse Transitions::try_state. Do not enlarge the budget or
relabel missing capture as unsupported; investigate repeated type observation
work/accounting before claiming this context. Independent unchanged-original
oracle443,721 checks/context remains green across four runtime contexts; foreign
native CI unrun. Next source-selected error-flow frontier: callable guarded
arithmetic Results, then required map_err/? composition; shl and higher parser
callers need their complete additional blockers audited. No new SHA256 or
serialized lookup keys. Natural capture27.18s/375,744KiB peak; focused composition
2.25s, not a controlled performance comparison. Broad goal active.
[Contract](../eval/coverage-development/wire-arithmetic-errors/README.md).
Evidence: `/tmp/rstyle-448-{comparison.json,strict-comparison.json,
frozen-comparison.json,nextest.log,focused-final.log,clippy-final.log,
style-final.json,patterns-final.json,natural-final.json,default-final.json}`.

Concrete observation reuse (Data3.119 unchanged): previous failure is confirmed
as4,097 cumulative entries at depth1. Exact rustc Ty caches reuse only completed
root observations/normalized tokens within one fixed function Capture; recursive
depth/visiting checks, first-pass origins, per-use selections and all limits remain.
The repeated-reference regression formerly failed capture; now all161 original
reads remain, while its unregistered call remains excluded. Oversized single trees
and repeated interface inventories still fail incomplete; root char cannot warm
nested char permission. Full2,181 tests/5 existing skips plus doctests pass;
fmt/Clippy clean, style0/7,257 and patterns0/15 unchanged ignores. No-default all590
function records,138 admissions/276 strict imports and9,453 pair records match
ceec464 exactly. Default assessment now completes:110/2,582; this is restored
capture, not a110-admission gain from a failed denominator.

The unchanged selected default originals still exclude: dense.rs imports alloc::vec
and cleanup ownership routing rejects that external path before existing pinned
library authority. Next prove exact library import routing while retaining unique
imports, shadowing, unknown globs and whole macro-argument traversal; no blanket
builtin fallback. Then source-select check_slice_len::<u8>→try_read_u32 with concrete
instance identity, unit/tuple Results, guarded static-error summaries and same-error
propagation. All seven direct add/mul callers need unsafe or stateful iterator/
mutable collection support; summaries alone would unlock none. Do not prioritize
shl solely for a smaller syntax fixture. Higher safe validators require additional
scalar payloads and ordered mismatch cases. Foreign native CI unrun; goal active.
Evidence: `/tmp/rstyle-449-{comparison.json,strict-comparison.json,nextest.log,
clippy.log,style.json,patterns.json,default-before.json,default-after.json,
no-default.json}`. Default capture104.76s/1,255,592KiB peak; no-default26.85s/
368,640KiB. No controlled speedup claim and no new SHA256/serialized lookup keys.
Frozen run-y8_t2xlf matches run-yvqmo2nk in every profile: all60 identities,
results/reasons/blockers/denominators and118 Data documents;24/29/7 unchanged.
Comparison: `/tmp/rstyle-449-frozen-comparison.json`.


Pinned library cleanup routing (Data3.120/148 capabilities): exact unique
alloc::{vec,format} and native std::thread_local imports now reach their existing
source contracts; full arguments/initializers, shadowing, globs, attributes and
pins still check. Original default wire arithmetic/error family connects;
regex-automata110→198/2,582,88 gains/no losses (51 previously supported without
defaults;37 incidental gains source-audited). No-default138/590 and every function
record unchanged. Strict672 original/replay imports,336 self-comparisons and
28,956 cross-pairs complete; all9,453 prior no-default pairs/verification records
identical. Default add/mul still structurally Accepted at1 despite unequal runtime
results: no equivalence/rewrite claim. Frozen run-v3l_x3bs preserves all60 identities,
results/reasons/blockers/denominators,24/29/7 and118 Data documents; other profiles
unchanged. Full2,188 tests/5 existing skips plus doctests, fmt/Clippy pass;
style0/7,258, patterns0/15 unchanged ignores. Cargo composition0.94s; original
source/build identities rechecked against the unchanged selection and retained
443,721-check/context runtime oracle. Foreign native CI unrun. No new runtime
hashing/serialized lookup/index pass. Default capture211.74s/1,294,168KiB and strict
replay401.91s/4,694,532KiB do more supported work; no-default capture27.10s/368,804KiB.
No controlled performance claim. Evidence: `/tmp/rstyle-450-{comparison.json,
strict-comparison.json,gains-audit.json,frozen-comparison.json,nextest.log,
clippy.log,style.json,patterns.json}`. Next: check_slice_len::<u8>→try_read_u32,
requiring concrete instance identity, unit/tuple Results, guarded callable error
origins and same-error propagation; then other widths and ordered validators.
General alloc/TLS effects, arbitrary macros and cleanup remain separate. Goal active.


Fallible-reader proof inputs (Data3.120/148 capabilities unchanged): original
regex-automata0.4.18 `check_slice_len::<u8>` → `try_read_u32` is source-selected
with full error/dependency roster and independent native oracle285,339 cases in
each default/no-default × debug-on/off context, zero mismatches. Observation-only
u8 owner overlay records13 typed nodes,1 concrete len method and6 actual call
receipts in no-default (19,901 minified bytes); original generic HIR, ordinals,
source_scan_complete=false and deferred len diagnostic remain intact. Unit and
`(u32,usize)` error interfaces now retain declarations without granting ownership.
Default198/2,582 and no-default138/590 unchanged, gain0/loss0; all original source/
build identities and admitted records persist. Excluded records gain complete
error declarations and richer rejection diagnostics; earlier declarations remain.
No capability/version increase, new runtime hashing or serialized lookup keys.

Exact-tree gates:2,200 tests pass/5 existing skips plus doctests; fmt/Clippy clean,
style0/7,296 and patterns0/15 unchanged ignores. Focused Cargo composition1.32s.
Natural captures213.04s/1,288,708KiB default and27.70s/367,744KiB no-default; these
are single observations, not controlled performance claims. Foreign native CI
unrun. Evidence: `/tmp/rstyle-451-{default,no-default}-comparison.json`,
`/tmp/rstyle-451-final-{fmt,clippy,nextest,style,patterns}.log`,
`/tmp/wire-fallible-readers-oracle.json`. Frozen run-n4m4nmj0 matches
run-yvqmo2nk: all60 identities/results/reasons/blockers/denominators,24/29/7 and
118 Data documents; other profiles unchanged. Comparison:
`/tmp/rstyle-451-frozen-comparison.json`.

Next connected delivery: terminal original try_read_u32 admission, with its
concrete length helper callable dependency. Reuse private InstanceProof/retained
owners; independently verify the entire original source/HIR, substitution and
pinned len authority, then cover only its exact deferred finding through
checked_activation::deferred_call and checked source-input ownership. Add bounded
unit/tuple named-error no-drop boundaries, real boolean-guarded message routes,
and same-error unit→tuple `?` reconstructing the output-typed Err. Keep saved
slice/read/panic evidence, including the existing read_u32 ByteWindow4 summary.
Do not broaden scalar instance authority, spoof the one-argument error-constructor
summary, or make the reader itself callable before this root composition works.
Resealed substitution/build/callee/guard/origin/ordering/cleanup adversaries and
strict original admission/matching are required before claiming support. This
foundational checkpoint follows the88-admission milestone; another observation-
only increment would require replanning. Broad goal active.
[Source contract](../eval/coverage-development/wire-fallible-readers/README.md).


Data3.121/149 capabilities connects original `check_slice_len::<u8>` →
`try_read_u32`: independent original-owner instance/source proof, complete call
receipts, guarded no-drop error output and unit→tuple same-error `?`. The native
reader's real reads/panic evidence remains. Generic originals and unsupported
opaque-loop callers remain excluded; inventory completeness grants no call-site
linking authority. The reader is a terminal admitted root, not yet callable.

Fresh default198→199/2,582 and no-default138→139/590: sole gain
`util::wire::try_read_u32`, zero losses, previous admitted records unchanged.
Strict676 original/replay imports,338 self-comparisons and29,292 cross-pairs
complete; all28,956 prior pair records and336 verification records persist.
Independent unchanged-source oracle285,339 cases/context × default/no-default ×
debug-on/off passes. Full2,222 tests/5 existing skips plus doctests; fmt/Clippy
pass, style0/7,510, patterns0/15 unchanged ignores. Focused Cargo composition2.33s.
Foreign native CI remains unrun; no detector-quality/equivalence/rewrite claim.

Targeted activation recaptures the O(V+E) reverse caller closure only when affected
functions have no prior roots/helpers and the owner has no private projections;
otherwise complete recapture remains. All original facts and prior PDG/CFG bytes
are tested unchanged. No new runtime hashes or serialized lookup keys. Controlled
same-work no-default samples (warmups, alternating two repeats/revision) preserve
all590 outcomes/139 admissions:36.75–38.40→29.84–30.26s,405.6–405.7→382.7–383.0MiB
peak. This is a local workload result. Natural default capture229.85s/1,745,924KiB
and strict replay201.42s/6,041,888KiB are single observations with more supported
work than the baseline; whole-unit replay memory remains a limitation. No-default
strict replay36.48s/1,302,868KiB. Replay reuses one policy-local analysis per owner,
while verifying each original and serialized pair independently.
Evidence: `/tmp/rstyle-452-{default,no-default}-comparison.json`,
`/tmp/rstyle-452-strict-comparison.json`, `/tmp/rstyle-452-perf-summary.json`,
`/tmp/rstyle-452-oracle.json`, `/tmp/rstyle-452-{nextest,clippy,fmt,style,patterns}-final.*`.

Next ready target: original callable reader → `read_version` and
`read_endianness_check`, delivered as connected callers. Required shared seams:
compose guarded tagged outputs with verified read/panic outcomes; support exact
tuple→scalar same-error propagation; retain ordinary scalar tuple let components
from one saved initializer; represent source-backed static literal roots; support
ordered two-scalar mismatch constructors; retain saved assertion operands/effects.
The driver currently emits childless unsupported tuple patterns, and existing
static origins only represent formals; neither can be bypassed. Keep historical
memory-free summary validators unchanged. Tests must reject swapped tuple slots/
expected-found operands, changed messages/residuals, omitted reads/guards, hidden
Drop and foreign owner evidence. Short-input BufferTooSmall precedes decoding or
mismatch; success consumes4. Wider readers alone do not complete these callers.
Broad goal active. [Contract](../eval/coverage-development/wire-fallible-readers/README.md).

Frozen reproduction `target/hir-coverage/run-mhlxtnst` preserves all60 identities,
outcomes/reasons/blockers/denominators and118 Data documents:24 admitted/29 excluded/
7 inactive. R0 and expanded profiles are also unchanged. Comparison:
`/tmp/rstyle-452-frozen-comparison.json`.

Data3.122 / guarded validator integration (150 capabilities). Original
regex-automata0.4.18 `try_read_u32` → `read_version/read_endianness_check` now
connects callable tagged read/panic summaries, saved scalar tuple bindings,
exact tuple→usize same-error `?`, static error roots and ordered two-u32 mismatch
constructors. Standard assertion scopes require retained resolution/source proof
and recursive argument closure; old policies and hidden-item rejection persist.

Unchanged selections: default199→203/2,582; no-default139→143/590. Exactly both
validators and both constructors gain admission, zero losses. Strict original/
serialized replay:692 imports,346 self-comparisons,30,656 complete cross-pairs;
all29,292 prior pairs persist. Frozen run-ifumaiut preserves every60 identity,
outcome, reason, blocker and denominator (24/29/7),118 Data documents and both
older profiles. Oracle285,339 cases/context × four native contexts, zero
mismatches; foreign native CI remains unrun. Full2,242 tests/5 existing skips
and doctests pass; fmt/Clippy/style0/7,619/patterns0 pass,15 ignores unchanged.

Capture default231.78s/1,779,916KiB; no-default30.13s/402,056KiB. Strict replay
208.48s/6,168,624KiB and42.31s/1,347,784KiB respectively; whole-unit replay
memory remains open. New fixture6.66s in the full run. Mutation checks use typed
verification, retaining original serialized replay; no new runtime hash/cache key.
Evidence: `/tmp/rstyle-453-{default,no-default}-comparison.json`,
`/tmp/rstyle-453-{strict,frozen}-comparison.json`, `/tmp/rstyle-453-oracle.json`,
`/tmp/rstyle-453-nextest-final2.log`, `/tmp/rstyle-453-{style,patterns}-final.json`.
Next: source-selected default-only `Flags::from_bytes`, requiring aggregate
success ownership/construction and tuple→aggregate propagation. Validator callers
are unsafe DFA deserializers; broader coverage and rewrite safety remain open.


Data3.123 / bounded aggregate reader integration (151 capabilities). Original
`try_read_u32` → `Flags::from_bytes` now admits with actual builtin Copy/no-drop
proof, exact tuple→aggregate propagation, saved field/count computations and
complete guarded error/read/panic origins. Default203→204/2,582: sole gain Flags;
no-default143/590 unchanged, zero losses. Strict694 imports/347 self-comparisons/
30,859 complete cross-pairs preserve all30,656 historical records. Frozen60
identities/outcomes/reasons/blockers/denominators,118 Data documents and older
profiles are unchanged (24 admitted/29 excluded/7 inactive).

Independent whole-original oracle82,368 cases × two default debug-assertions
contexts passes; foreign native CI is unrun. Full2,250 tests/5 existing skips,
doctests, fmt/Clippy, rstyle0/7,660 and patterns0 pass;15 ignores unchanged.
New fixture7.83s in the full run; typed adversaries retain original serialized
replay without repeated seals. No new runtime hashes or serialized cache keys.
Capture default231.85s/1,797,360KiB; no-default30.21s/401,892KiB. Strict replay
224.61s/6,213,304KiB and41.96s/1,347,744KiB respectively; whole-unit replay memory
remains open. Evidence: `/tmp/rstyle-454-{default,no-default}-comparison.json`,
`/tmp/rstyle-454-{strict,frozen}-comparison.json`, `/tmp/rstyle-454-oracle.json`,
`/tmp/rstyle-454-nextest-final.log`, `/tmp/rstyle-454-{style,patterns}-final.json`.

Next source-audited candidate: original wire `read_state_id`,
`try_read_state_id`, `read_pattern_id` and their two error constructors in both
unchanged contexts. Decoder summaries and generated-error no-drop proofs already
exist; confirm their selected call/import and associated-SIZE receipts before
redesigning. Remaining chain: capturing `map_err`, one-use owned error + static
string construction, nominal tuple-success propagation and complete bounds/read/
panic composition. Direct readers must retain short-input panics; the checked
wrapper returns BufferTooSmall first. Higher consumers add mutable stored
closures/cursor state or loops. Freeze source identities/expectations and resolve
remaining receipt/type-observation unknowns before production edits. Broad goal
remains active; no general moves/drop, unsafe, detector-quality or rewrite claim.
