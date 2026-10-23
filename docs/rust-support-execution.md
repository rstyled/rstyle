# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-23. All P01–P22 routes
remain active in the plan; this record contains current decisions and evidence.

## Current boundary

Branch `feature/rust-feature-coverage`; Data3.92 connects both original
`Match::new<Span/Range<usize>>` private owning instances. Fresh default/no-default
captures retain590/2,582 definitions on each declared target; both instances pass
strict replay and all six selected original/replay pairs. Ordinary generic caller
linking remains closed. The next memory target is frozen safe lz4_flex0.14.0
`backtrack_match`; its independent private-body oracle passes48,856 cases, while
location-aware loads/stores and loop post-state remain unsupported.
The root roadmap records validated checkpoints; broader P01–P22 work remains
active and historical evidence stays unchanged.

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
