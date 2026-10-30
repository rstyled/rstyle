# Original wrapped LineReader chain

`selection.json` freezes base64ct1.7.3 `LineReader::new_wrapped`, its unchanged
`new_unwrapped` helper, complete owner/neighbors/imports, and `MIN_LINE_WIDTH`.
This separate development selection leaves earlier inventories unchanged.

The independent expectation is InvalidLength exactly when width<4 or input is
empty; success preserves the input backing pointer/length/content and stores
Some(exact width). The width guard precedes helper evaluation, and helper failure
precedes initialization/update. Required proofs connect the complete guarded
helper Result, same-product/error `?`, and an initialized nonescaping no-drop
owner with Copy field updates followed by one whole move. Aliases, escapes,
reference-field updates, partial/duplicate moves and hidden cleanup remain closed.

Run `python3 tests/fixtures/function-families/line-reader-wrapped/oracle.py` to compare
exact original ranges and existing historical identity references without hashing.
Add `--run-native` for the assembled exact-private-source witness: 64,638 cases in
each default/std context, with independently specified width/error/backing values.
This witness is separate from unchanged whole-crate capture, strict owning-bundle
replay, selected matching, and adversarial graph/source verification. Source
expectations were frozen before admission evaluation. All six original contexts now
admit (gain1/lose0 each), retaining complete141/150 function rosters. Strict replay,
selected identity matching and108 resealed semantic attacks pass; equivalent Cargo
wrappers match and an ownership escape rejects. Full-library matching remains
incomplete. See the [bounded contract](https://github.com/rstyled/rstyle/blob/c35bd14c/docs/extraction-profiles/linear-product-result-flow-v1.md)
for gates and remaining limits; native x86/macOS execution is not claimed.
