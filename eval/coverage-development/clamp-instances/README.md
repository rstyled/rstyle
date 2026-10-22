# Original scalar clamp instance readiness

This separate source-first development selection freezes unchanged `num-traits
0.2.19` originals `clamp` (src/lib.rs:402), `clamp_min` (:422), and `clamp_max`
(:440), each instantiated at i32/u32. It preserves the Input selection and every
frozen inventory. The native oracle passes; extraction, admission and matching
remain separate pending gates.

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

Remaining blockers are concrete instance identity/substitution and replay;
source/interface binding for type generics; exact primitive PartialOrd/PartialEq
selection and overloaded-to-scalar operation proof; root module imports, macro
expansions and shadowing closure; and complete assertion/result composition.
Original `clamp_min/max` include `#[allow(clippy::eq_op)]`; all three have
`#[inline]` and a `debug_assert!` with a literal diagnostic. Preserve these and
use the existing bounded assertion authority/build checks. The float-only,
nongeneric scalar-leaf exception does not cover this family. No ADT, reference,
AsRef, inherent/blanket impl ownership, or non-Copy cleanup support is needed for
the selected scalar instances. Full neighboring source proof is still required.

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
Instance capture, strict verification/admission/matching, adversarial evidence
mutations and unchanged-selection admission deltas remain required future work.
