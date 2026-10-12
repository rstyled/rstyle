# Inventory protocol corrections

The initial protocol incorrectly spelled the host as x86_64-unknown-linux-gnu.
The installed pinned compiler is aarch64-unknown-linux-gnu, commit
5c543b0b8c73c7b72bc8284ced4fb22ead15734d, pointer width 64. Initial capture used
that actual host. Those observations are disclosed development inventory evidence,
not a run satisfying the initial target spelling. The corrected frozen target
for subsequent before/after inventory runs is aarch64-unknown-linux-gnu; package,
source selection, sampling and features are unchanged. There are no detector
predictions or quality labels in this inventory.

Unmodified registry packages initially placed under the repository's target tree
failed Cargo's enclosing-workspace check. Moving the identical packages outside
the repository resolved packaging without source/manifest edits. Initial failed
stdout/stderr and successful capture artifacts remain in the local frozen-package
and inventory directories. Committed result records retain report identities.

The initial conditional attempt exposed incomplete external-module pre-expansion
scans, retained as conditional-attempt0.json. Later conditional.json/loops.json
captures preceded the parenthesized-expression correspondence correction;
conditional-final.json/loops-final.json record the first successful correction.
These are disclosed development inventory attempts, not separate holdouts.

The conditional-release rerun failed lz4 compilation planning because a concurrent
workspace build replaced the running cargo-rstyle executable; current_exe then
resolved to a deleted path. The failure and stderr are retained. It is capture
failure, not an eligibility rejection. Final validated inventory captures were
run after workspace builds/tests completed, with no competing binary rebuild.
The selected packages, source bytes, targets/features and sampling rules did not
change. Parenthesized boolean loop literals subsequently gained an adversarial
regression fix without changing the frozen source selection or matching policy.
