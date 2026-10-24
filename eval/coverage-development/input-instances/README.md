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

Data3.85 activates the original byte constructor in both native feature contexts
through a separate checked owner and strict reference sidecar. Each unchanged
four-instance selection gains one admission with no losses; str and both From
instances remain unsupported. The generic definition rosters remain2,582/590.
Complete original cleanup and outer From association proofs are now available;
From still needs checked body/call materialization and component summaries.
[Bounded activation contract](../../../docs/extraction-profiles/reference-constructor-instances-v1.md).
After a fresh capture, reproduce strict replay, producer admission, identity
matching and graph attacks with prebuilt binaries:

```sh
cargo build --locked -p rstyle-ccgraph --example reference_constructor_activation
for context in no-default default; do
  timeout 240 target/debug/examples/reference_constructor_activation \
    /tmp/input-owner-diagnostics/"$context"/snapshots/*-pre-helper-certification.json \
    /tmp/input-owner-diagnostics/"$context"/snapshots/*-reference-instances.json
done
```

The default/no-default full-owner replay probes took98.93/13.57s and peaked at
1,213,460/208,636KiB on the native runner; these include complete owner analysis,
not just the constructor. Identity matching is a control, not detector-quality
evidence. Cross-target activation and actual foreign caller linking remain pending.

Data3.86 adds the string constructor with an explicit UTF-8 byte-view origin.
The same probe now requires both constructors and independently checks their
single returned reference component: Input field0, shared `[u8]`, formal0, with
`SharedReferent` for bytes and `SharedReferent → Utf8Bytes` for strings. Both From
instances remain unsupported. This brings each unchanged native selection to
2/4 admitted; no cross-target or foreign-caller admission is claimed.

Data3.87 connects both original From callers to the exact checked constructor at
the same substitution. Both native contexts admit4/4 on the unchanged selection,
with2 gains/0 losses from3.86. Strict nested-proof replay, producer admission,
component-origin expectations, identity matching and call/dependency/policy
attacks are required by the same probe. Replay reuses an inspected immutable
owner, and nested proof inputs avoid JSON re-encoding. Ordinary foreign caller
linking is still pending; generic definition rosters are unchanged.
