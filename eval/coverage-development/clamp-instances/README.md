# Original scalar clamp instance readiness

This separate source-first development selection freezes unchanged `num-traits
0.2.19` originals `clamp` (src/lib.rs:402), `clamp_min` (:422), and `clamp_max`
(:440), each instantiated at i32/u32. It preserves the Input selection and every
frozen inventory. The native oracle and bounded scalar instance admission/replay now pass;
contextual caller matching is measured separately from original-source support.

There are four feature/assertion contexts: default (`default,std`) and no-default
(no features), each with debug assertions on/off. All use dev opt-level 0,
overflow checks, unwind, and the pinned toolchain. This isolates assertion mode
from optimization. `libm` is off. The source target roster is Linux ARM64, Linux
x86_64, and macOS ARM64; native execution and cross-target capture remain separate.
The build script's `has_total_cmp` outcome must come from actual Cargo capture.

`selection.json` stores exact selected lexical bytes (including attributes),
manifest/build-script bytes, independent classifications and expectations. These
records do not certify unrecorded neighboring source. Cargo must resolve the
unchanged registry dependency, not a copied body or path replacement. No new
hash/lookup registry is introduced.

The native oracle calls the actual library. It exhausts 17-value triple domains,
256-value pair domains, and full-width integer edge combinations. Reversed bounds
panic only with assertions enabled; disabled assertions retain the original
min-first branch order. Equal bounds and signed/unsigned extremes are included.
The expected count is 272852 scalar calls per context, 4921 expected caught panics
with assertions enabled (zero otherwise), and six custom controls. Effectful
PartialOrd controls record comparison order; nonreflexive controls demonstrate
why arbitrary traits stay unsupported. Custom controls are native semantic
counterexamples, not proposed admissions.

The supported contract is one original free function with one type parameter,
all inputs/result that parameter, instantiated at i32/u32. Exact primitive
comparison implementations, original generic bounds, assertion origins and the
complete lexical source owner are checked independently. Caller links also
require the actual Cargo dependency graph, renamed --extern edge, emitted and
loaded artifact identities, and structural requested/selected arguments.
Original definitions retain their ordinals and generic source; checked instances
live in separate owners. This does not support arbitrary traits, other scalars,
reference/aggregate substitutions, generic methods or different result types.

Verified native commands:

```sh
python3 eval/coverage-development/clamp-instances/oracle.py --verify-only
python3 eval/coverage-development/clamp-instances/oracle.py \
  --target-dir /tmp/rstyle-clamp-instances-oracle-target \
  --output /tmp/rstyle-clamp-instances-native-oracle.json
```

`--package-root` relocates the original registry checkout, and `--target-dir`
selects a build cache. The runner verifies selected bytes, registry resolution,
features, debug mode and native results. Its output is native evidence only.
`compose.py --output /tmp/rstyle-clamp-connected` captures the unchanged registry
library in all four contexts, then strictly reconstructs the dependency closure.
`callers.rs` supplies 12 composition peers and one independently excluded u8
control; these wrappers are not original natural functions or held-out examples.
All 48 contextual callers admit, all four u8 controls stay excluded, and all 264
pair evaluations complete, including the six expected identical peer pairs per
context. Original owner rosters remain 2,212 default / 1,965 no-default definitions;
raw generic definitions remain excluded. Separately, all 24 original checked
instances produce admitted graphs and replay from proof-only sidecars. No new
hashing or serialized lookup keys are introduced.

Retained local evidence: `/tmp/rstyle-clamp-connected/{summary,owned-replay}.json`.
Fresh captures take 5.4–6.0s/context and strict closure replay 1.9–2.1s/context on
this Linux ARM64 runner. These are observed timings, not controlled speedup
claims. Native x86_64/macOS execution and a fresh frozen-inventory comparison
remain pending; historical inventory identities and denominators are unchanged.

The owning compiler now publishes complete bounded projection request outcomes
in the existing unit bundle. Unknown/incomplete/duplicated requests fail closed;
unsupported/unresolved/error/unavailable requests retain their distinct reasons.
Only a private independently checked instance token activates concrete generic
source. Sidecar replay carries proof inputs, never graphs or serialized authority.
