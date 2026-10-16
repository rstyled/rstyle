# Typed declaration origins

Following `9c05c40`, declaration origins join the closed set of typed frontend
records. The driver moves `Origins` directly into `FrontendOptions`; nominal
binding and nominal resolution borrow it. Import decodes this record once into
its owning options table. Neither consumer constructs a JSON tree or clones and
redecodes the record merely to access it. JSON mutation adapters invalidate typed
ownership and frozen-unit normalization restores it, or retains the malformed
record and decoding error.

This is an internal representation change. The declaration-origin/1 wire schema,
sorted JSON encoding, explicit null handling, exact file bytes, declaration
ranges, digest claims and validation remain unchanged. Borrowing a typed record
confers no verification authority. Public origin checks and owner-bound source
environment checks still run in their respective callers.

The regression checks original buffer addresses across 100 borrows, lossless
wire round trips, compatibility edits, normalization and malformed spans. This
is structural evidence of eliminated copies/decodes, not a wall-time speedup
claim. Origin file buffers still duplicate census snapshots; public verification
still serializes origins for its size budget and checks source fingerprints.
These remaining costs are outside this increment and remain on the task list.

Validation: 65 library unit tests passed; warning-free workspace/all-target
Clippy; full pinned nextest wrapper passed 655 tests, zero skipped, followed by
workspace doctests. Formatting and diff checks passed. Both quality gates passed:
zero style violations (2,003 functions), zero actionable pattern candidates,
15 existing reviewed ignores unchanged. No suppressions or thresholds changed.
Logs are `/tmp/rstyle-origins-{unit,clippy,full}.log`,
`/tmp/rstyle-origins-style.json` and `/tmp/rstyle-origins-patterns.log`.
