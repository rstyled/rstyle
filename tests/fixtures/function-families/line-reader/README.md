# Original LineReader constructor

This separate source-first selection freezes base64ct1.7.3
`decoder::LineReader::new_unwrapped` and its complete original owner before
admission evaluation. Historical source identities are reused; the checker
compares exact frozen bytes/ranges without computing new hashes. Existing
selections and the frozen sixty remain intact.

Expected behavior: **Err(InvalidLength) iff input is empty**. Otherwise return a
fresh non-Copy reader with exactly the input backing pointer/length/content and
`line_width=None`. There is no byte-content read, allocation, mutation or panic.
The selection retains every known source, no-drop, Option-field, fresh aggregate
Result and guarded component-origin obligation, negative expectations and actual
caller blockers. Neither public Decoder constructor becomes supported from this
method alone.

The native oracle assembles the exact original private owner/method, adapts only
the public Error import and links the unchanged original dependency. This is a
private-source witness, separate from unchanged full-crate capture/admission.
It makes9,234 constructor calls per default/std context: lengths0–512, three
backing offsets, three patterns and direct/temporary-reference-scope calls.
Expect9,216 successes and18 InvalidLength errors per context, zero mismatches.
Native execution covers only its host; original acceptance requests six contexts
(default/std across the three pinned targets). The initial freeze makes no admission claim.

Source-only check, without building:

```sh
python3 tests/fixtures/function-families/line-reader/oracle.py
```

Run after the coordinator's other builds/gates finish:

```sh
python3 tests/fixtures/function-families/line-reader/oracle.py --run-native \
  --target-dir /tmp/rstyle-line-reader-native-target \
  --output /tmp/rstyle-line-reader-native-oracle.json
```

Each Cargo run has a120-second default timeout. Successful native execution does
not change the pending original capture, strict replay or selected matching gates.

Native checkpoint: pinned ARM Linux default/std witness passes9,234 queries each
(9,216 successes/18 InvalidLength errors),0 mismatches. Evidence:
`/tmp/rstyle-line-reader-native-oracle.json`. All six retained complete original
Cargo contexts from Data3.88 exclude this method with exact source/span matching;
`/tmp/rstyle-line-reader-baseline.json` records this baseline. The baseline is retained unchanged.

Data3.89 delivers the selected original in all six contexts: gain1/lose0 each,
0/1→1/1, full rosters141/150. Strict owning-bundle replay, six selected identity
comparisons and36 resealed origin/None/field-slot/operand/policy attacks pass.
The native oracle was rerun with0 mismatches; cross-target capture does not imply
native macOS/x86 execution. Full workspace gates pass (1,483 tests/five existing
skips, doctests/fmt/Clippy, rstyle0/4,743, patterns0/15 unchanged ignores).
See the [bounded contract](https://github.com/rstyled/rstyle/blob/c35bd14c/docs/extraction-profiles/fresh-reference-product-results-v1.md).
Evidence: `/tmp/rstyle-389-{reader-evidence,reader-native-oracle}.json` and gate logs.
`new_wrapped` still needs guarded helper export, product `?` and linear local
field-update/move evidence; public Decoder support remains unavailable.
