# Inventory protocol corrections

The initial protocol spelled the host as x86_64-unknown-linux-gnu. The installed
pinned compiler is aarch64-unknown-linux-gnu, commit
5c543b0b8c73c7b72bc8284ced4fb22ead15734d, pointer width 64. The corrected target
for the reported before/after runs is aarch64-unknown-linux-gnu; package selection,
source bytes, sampling and features did not change. No detector predictions or
quality labels were inspected.

Packages initially extracted under the repository target tree failed Cargo's
enclosing-workspace check. Reproduction now uses temporary directories outside
the repository without source/manifest edits. The final reported captures were
run without competing executable rebuilds; earlier attempts included capture
failures and incomplete correspondence. Those development attempts are not
additional heldouts or coverage measurements.

Development outputs are now regenerated locally rather than committed. The
report records the tested implementation commit and measured outcomes. Frozen
inputs and dependency lockfiles reproduce the selection/build context; generated
assessments preserve every reason, unknown and failed outcome. No historical
ccgraph-v1/v2 artifacts were removed or relabeled.
