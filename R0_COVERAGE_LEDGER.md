# R0 coverage ledger (`compat.rust.hir-r0 / 0.1.0`)

Independent record of what the HIR R0 producer has demonstrated, what
remains, and under which bounds conformance claims hold. Pinned contracts
are never edited here; this file only cites them.

## 1. Authority and baseline

- Authoritative compat revision: `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`
  (vendored byte-for-byte under `vendor-compat/rust-r0/`, see `ORIGIN.md`).
- Targets: extraction profile `compat.rust.hir-r0 / 0.1.0`, PDG `0.1.0`,
  CFG `0.2.0`.
- Landed imported-contract-hardening revision: `a9da3eb`
  (`fix(patterns): merge import partition loop, dismiss reviewed candidates`,
  tip of `ccgraph/imported-contract-hardening`), ancestor of this branch.
  Its strict admission adapter (`crates/rstyle-ccgraph/src/admission.rs`)
  and PDG validation boundary (`crates/rstyle-pdg/src/validate.rs`,
  `crates/rstyle-pdg/src/cfg.rs`) are present and all workspace tests pass.
- Branch: `ccgraph/hir-r0-producer`. Legacy extraction/matching behavior is
  unchanged; no legacy graph is relabeled R0-conformant.
- Producer assertions alone never establish eligibility: every accepted
  document must cross the hardened validation/admission boundary, and every
  comparison must run against the independent expectations in
  `crates/rstyle-r0fixture` (never derived from producer routines).

## 2. Capability status

`unimplemented` = no producer evidence. `in_progress` = partial,
boundary-described work. `verified` = producer output compared exactly
against independent expectations **and** admitted through the hardened
boundary. Schema acceptance alone never advances a status.

| Capability ID | Status | Evidence / boundary |
| --- | --- | --- |
| `rust.scalar-interface` | unimplemented | Expectation side parsed + selectors resolve (`rstyle-r0fixture`). No producer type evidence. |
| `rust.local-bindings` | unimplemented | Same as above. |
| `rust.local-writes` | unimplemented | Same as above. |
| `rust.scalar-computation` | unimplemented | Same as above. |
| `rust.blocks` | unimplemented | Same as above. |
| `rust.branching` | unimplemented | Same as above. |
| `rust.short-circuit` | unimplemented | Same as above. |
| `rust.returns` | unimplemented | Same as above. |
| `rust.control-dependence` | unimplemented | Same as above. |
| `rust.execution-dependence` | unimplemented | Same as above. |
| Deferred (`rust.call-summaries`, `rust.abstract-memory`, `rust.loops`, `rust.match`, `rust.for-iteration`, `rust.closures`, `rust.macros`, `rust.async`, `rust.unsafe`, `rust.unwinding-cleanup`, `rust.unspecified-order`, `rust.additional-types`, `rust.nonreturning-calls`, `rust.divergence`, `rust.conservative-call-effects`, `rust.precise-aliasing`) | unimplemented | Negative fixtures parsed; rejection diagnostics carry capability IDs. No producer rejection evidence yet. |

## 3. Bounded-conformance decisions

- **Record-level policy omission.** The pinned PDG schema permits optional
  record-level `policy` objects referencing the R0 profile. The workspace
  transport (root-policy simplification, `vendor-compat/pdg.schema.json`)
  rejects record-level `policy` fields, and the hardened boundary enforces
  that. The producer omits them and is governed by the root `profile`
  (`compat.rust.hir-r0 / 0.1.0`) instead. Omitted fields validate as absent
  under both schemas; no divergent policy is asserted.
- **Fixture-local proofs deferred.** Cases requiring exceptional-flow
  impossibility proofs (e.g. `rhs-first-primitive-compound-write`,
  `ordinary-binary-old-value-snapshot`) record their concrete local proof
  text in the fixture; the producer has not reproduced any such proof yet.
- **Initial pack is not certification.** The 23 vendored cases are the
  comparison oracle for this milestone, not exhaustive proof of every R0
  operation or deferred capability. `supported`/`conformant` claims stay
  scoped to demonstrated cases.
- **Reference implementation tests are not authority.** Existing legacy
  graphs, passing implementation tests, and moving compat branches never
  override the pinned contracts on conflict.

## 4. Fixture cases and producer status

`not_run` = the actual producer has not exercised this case yet.
Identities are `sha256` of the exact source bytes (no added newline),
recomputed by `rstyle-r0fixture`.

| Case | Expectation | Producer |
| --- | --- | --- |
| straightline-integer-input-and-tail | eligible, exact, CFG partial (overflow panic omitted) | not_run |
| two-arm-reassignment-reaching-set | eligible, exact, CFG complete | not_run |
| short-circuit-and-saved-left | eligible, exact, CFG complete | not_run |
| short-circuit-or-outcomes | eligible, exact, CFG complete | not_run |
| early-return-controls-continuation | eligible, exact | not_run |
| returning-arm-does-not-execute-continuation-declaration | eligible, exact | not_run |
| if-value-effectful-declaration-site | eligible, exact | not_run |
| saved-left-value-before-branching-operand | eligible, exact | not_run |
| nested-if-merges-share-site-with-distinct-availability | eligible, exact | not_run |
| rhs-first-primitive-compound-write | eligible, exact, fixture-local overflow-freedom proof required | not_run |
| ordinary-binary-old-value-snapshot | eligible, exact | not_run |
| parameter-shadow-initializer-reads-old-binding | eligible, exact | not_run |
| empty-unit-body-retains-unused-interface | eligible, exact | not_run |
| unit-binding-retains-definition-not-value-operands | eligible, exact | not_run |
| explicit-unit-literal-return | eligible, exact | not_run |
| unreachable-supported-source-preserved | eligible, exact, reachability + diagnostic edges | not_run |
| unsupported-unit-comparison-not-builtin | ineligible (`rust.scalar-computation`) | not_run |
| unsupported-direct-call | ineligible (`rust.call-summaries`) | not_run |
| unsupported-call-after-return | ineligible (`rust.call-summaries`) | not_run |
| unsupported-short-circuit-macro | ineligible (`rust.macros`) | not_run |
| unsupported-nested-item-owner | ineligible (`rust.blocks`) | not_run |
| unsupported-unnamed-parameter | ineligible (unnamed parameter) | not_run |
| unsupported-reference-interface | ineligible (reference interface) | not_run |

Source identities (`sha256` of exact bytes):

| Case | Identity |
| --- | --- |
| straightline-integer-input-and-tail | `sha256:b5ea3ee7b934b92de243704c845be5e33602f25909a73d509a63ee98926fc7bf` |
| two-arm-reassignment-reaching-set | `sha256:64647ff54e9eeaa3f5a74566951bf83412f0f4804fe54fd15138344013c6ce47` |
| short-circuit-and-saved-left | `sha256:18a069dc5f46744d06c63c09565b609b332d1a1b5e494a591540ee96d19cc9d5` |
| short-circuit-or-outcomes | `sha256:fc9aec7f1ae63f8f3f0aa6340bf5694f245a92f0781d6351237ca7eae3d4a646` |
| early-return-controls-continuation | `sha256:4101b865c8a04130768be6409566ec88e7321e66af72d8c423e9a5ac1f742e72` |
| returning-arm-does-not-execute-continuation-declaration | `sha256:184714834f2e7a9d2624b329bbe8314d8935b6c2a7aa304721173a80dd322358` |
| if-value-effectful-declaration-site | `sha256:e1453e908393cdb99e3d898735c826795a4922d8d2dfd04fdcb3763f7db1d6fa` |
| saved-left-value-before-branching-operand | `sha256:17e9d46e1568a3506e30141942330b6d8856726cad12206eaf4816d955a2b3af` |
| nested-if-merges-share-site-with-distinct-availability | `sha256:de1b0919218cd72ca8d47996c1cd5c67f3da5d683319ce8cce7068d6ea716623` |
| rhs-first-primitive-compound-write | `sha256:fb756ae61c97ce8337efa7f09117d53dd669f713ff02e1d6414754cc2ca9683c` |
| ordinary-binary-old-value-snapshot | `sha256:23d35ac8bfcfd0a0a828659767887c16591e568c7ce170f58c3168c7203ca582` |
| parameter-shadow-initializer-reads-old-binding | `sha256:69ae96b3dd0149d2f6161493860735acee50241ff27d2eb4b3c097d27bf9c227` |
| empty-unit-body-retains-unused-interface | `sha256:d43610f9a2305442c2f11415cdbd0197570660a73b00b1e7bf34929068c07e4d` |
| unit-binding-retains-definition-not-value-operands | `sha256:ccbfcb46b35e7f2fdfe55da6e08bb72185acbdf609f2654bf4a49cc34176c194` |
| explicit-unit-literal-return | `sha256:659440c6fa903ed945787d923029d7e52f964dded40a9` |
| unreachable-supported-source-preserved | `sha256:7ffcbf92e62fb3852b56e39378b86ebf16c1d15d6d51f76d850665147284eb42` |
| unsupported-unit-comparison-not-builtin | `sha256:b737f403497b6edd37211806b9cdf71836dab2c6a27e330d916f949d45281cff` |
| unsupported-direct-call | `sha256:6188228d385e56a65b828bf3c8045f8f75b4c6dca4697cb48d73c466c0e5dd23` |
| unsupported-call-after-return | `sha256:0b71e167737e91ea9bc35800df1b5d5c1447f9da615b9b544511d8d45cafa278` |
| unsupported-short-circuit-macro | `sha256:cc285b6793c98b2c01f7330a4015053b7ea0638dcabbfaad7fa32144a108cabe` |
| unsupported-nested-item-owner | `sha256:a2a1b388d978d42d8cbc840dd2a07cfa72dd2ef5de02a52f1650ed89441b194a` |
| unsupported-unnamed-parameter | `sha256:bddd05278a459732a1ead8cd34454cf55dd5398d478125294fd6adf8402f7e71` |
| unsupported-reference-interface | `sha256:27edccddb7ea5560998d297506f6e8115211d99b8c887b57d11fb11f08cb78b1` |

## 5. Remaining obligations

- [ ] Frontend evidence: run the pinned rustc over exact fixture source
      bytes; resolve HIR, types, and lexical bindings (slice 2).
- [ ] Strict eligibility scan with capability diagnostics, including
      unreachable/short-circuited/cfg-disabled source (slice 2).
- [ ] Operation CFG construction with exact lowering rules (slice 3).
- [ ] Reaching definitions, operand snapshots, postdominance control,
      nearest-dominator execution, merge guards/frontiers (slice 4).
- [ ] Schema-faithful paired PDG/CFG emission with build evidence (slice 5).
- [ ] Semantic validation through the hardened boundary; exceptional-flow
      limitation reporting; fixture-local proofs (slice 6).
- [ ] Adversarial/regression coverage: tamper rejection, ID/array
      invariance, determinism, producer-handoff test (slice 7).
- [ ] Out of scope (not started here): source-to-pair integration, bounded
      quality study, R1, matching-profile changes.
