# Concrete Input constructor instances

Source-selected originals: regex-automata0.4.18 `Input::new<H>` and its blanket
`From<&H>::from`, each with `H=[u8]` and `H=str`. The four-instance development
selection is separate from existing generic definitions and frozen inventories.
It was specified before evaluating instance admission. An absent instance is
unrepresented, not a captured exclusion or a denominator adjustment.

The native oracle uses the unchanged registry dependency under default and
no-default features. It checks exact slice pointer/content, byte-length spans,
all defaults, distinct equal-content buffers and saved subslices. Stateful,
static-storage and panicking AsRef implementations independently demonstrate why
the trait bound supplies neither purity nor a formal-rooted origin. Conversion
must run exactly once. Original `Input` is non-Copy but has no drop glue; this
observation alone does not establish its source/cleanup contract.

```sh
python3 eval/coverage-development/input-instances/oracle.py --verify-only
python3 eval/coverage-development/input-instances/oracle.py \
  --target-dir /tmp/rstyle-input-instances-oracle-target \
  --output /tmp/rstyle-input-instances-native-oracle.json
```

Source verification reuses the existing owner selection's historical pins once,
then compares the selected original byte ranges exactly. It introduces no new
hashing scheme, production hashing or transport seal.

The connected blocker chain is concrete instance/body substitution → exact
AsRef implementation/effects/origins → generic static/blanket source ownership →
complete non-Copy Input result and mixed Anchored ownership/component transport →
foreign reference/aggregate summaries. The complete `Pattern(PatternID)` variant,
SmallIndex, original derives/macros/imports and lifetime obligations remain in
scope even though the constructor chooses `Anchored::No`. Current scalar foreign
linking and borrowed Input getters do not establish this constructor chain.

Initial instance observation is evidence only: original generic HIR, source
spans and definition denominators remain unchanged. Dependency metadata cannot
supply its original local HIR/typeck, and region erasure cannot prove lifetime
or origin validity. Native behavior, cross-target capture, strict admission and
matching are separate validation gates. Broad P01–P22 work remains active.

Data-v3 key `rstyle.rust.call-instances` inventories direct function-item and
method calls with type/const arguments or trait members. It excludes closure/
function-pointer calls and nested owners; `complete` refers only to that scope.
Each site retains its HIR/span and requested/selected definition and arguments.
Structural identities cover builtin, str, reference, slice, tuple and nominal
ADT forms with actual compiler owner identities, bounded to256 nodes/32 depth.
Other type/const forms are explicitly unsupported; unresolved and compiler-error
outcomes stay distinct. Signature/display strings are forensic, never instance
identity or proof. Historical profiles have no such observation inventory.

Observed checkpoint: all four requests resolve in six original-package build
contexts; all owning bundles strictly import. The four call-site probes remain
excluded in each context, with zero admission gains/losses against the unchanged
baseline. Native oracle:132,736 constructor observations per feature context,
zero mismatches. Evidence: `/tmp/rstyle-instance-input-validation.json` and
`/tmp/rstyle-input-instances-native-oracle.json`. The instance inventory supplies
no generic-body or foreign-helper admission authority.

Owning diagnostics now observe all four original instances in both native feature
contexts, retaining original impl/type/lifetime binders, inherited clauses,
absolute source anchors, exact selected AsRef implementations and receiver
adjustments, and the From-to-constructor link. The 2,582/590 original definition
rosters are unchanged. Run against prebuilt binaries:

```sh
python3 eval/coverage-development/input-instances/capture.py --output /tmp/input-owner-diagnostics
```

This intentionally uses capture-only mode: no complete bundle or admission is
published. Independent verification must bind to the canonical full raw owner,
preserving its diagnostics, source census and declaration origins. Test-only
cleanup witnesses cover the complete modeled product and resealed drop/Copy
attacks; cleanup of the actual complete original module remains pending. Outer
From trait/impl association and owned component summaries are also still required.
