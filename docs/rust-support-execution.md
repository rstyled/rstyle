# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-23. All P01–P22 routes
remain active in the plan; this record contains current decisions and evidence.

## Current boundary

Branch `feature/rust-feature-coverage`; Data3.83 connects bounded original scalar
generic instances through exact cross-package call links and strict replay,
following Data3.82 calendar wrappers and the Data3.81 lz4 block-header parser.
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
