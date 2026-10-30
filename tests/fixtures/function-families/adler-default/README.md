# Adler default constructor chain

`adler2 2.0.1` original `Adler32::new` → `<Adler32 as Default>::default` is a
new two-function development family. The earlier
[checksum-state contract](../../../../docs/extraction-profiles/checksum-state-roundtrips-v1.md)
already delivers `from_checksum`/`checksum` and explicitly excludes this chain;
those methods are controls, not new gains.

`selection.json` freezes exact source bytes/ranges and XXH3-64 consistency
checksums for both functions, declarations, complete Default impl, manifest and
complete lib/algo context. Default std and no-default no_std contexts remain
separate; native ARM Linux execution and ARM Linux/x86 Linux/ARM macOS capture. Existing
frozen inventory selections and denominators stay unchanged.

`python3 tests/fixtures/function-families/adler-default/oracle.py` checks original
bytes using system libxxhash's XXH3_64bits, without builds. The coordinator may
schedule `--run-native` (optionally `--context default` or `no-default`). This
asserts original public new/default identity checksum1 and existing getter
controls. A separate diagnostic copy preserves original declarations and bodies
and adds sibling field witnesses to assert exact `(a,b)=(1,0)`. Familiar-name
controls demonstrate a different Default body and inherent default shadowing.
These controls do not certify arbitrary trait calls.

Data3.94 connects both original functions through the actual selected local
Default body and existing zero-argument Copy-product helper summaries. The
bounded contract requires the reviewed core Default/default member, an original
safe nongeneric single-member impl, a reference-free positively proved Copy
nominal result, direct prelude/owner binding and an original zero-argument call.
Independent raw call inventory, concrete Self, selected impl/member/signatures,
complete source anchors and ordinary body/effect verification must agree.
Explicit imports, QSelf calls, generic owners, owned/drop results and general
trait/default-method dispatch remain outside this contract. No constant or purity
summary is inferred from the name Default.

The independent native oracle passes eight original public API cases, two copied
private field witnesses and three dispatch controls in each std/no_std context.
Full-owner captures, strict original/replay matching and final checkpoint evidence
are recorded in [the execution log](../../../../docs/rust-support-execution.md).
Foreign-target capture does not establish native x86/macOS execution. Resealed
selection/inventory attacks, shadowed/custom traits, inherent lookalikes, generics,
Drop owners, disabled preludes and historical-policy downgrades remain negative
expectations. Existing checksum/getter admissions are regression controls.
