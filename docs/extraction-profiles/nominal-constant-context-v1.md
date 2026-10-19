# Nominal APIs beside constant declarations

Data-v3 extraction 3.11.0 adds `rust.nominal-constant-context`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The selected-unit nominal census now accepts
ordinary constant items and constants in supported inherent impls. Earlier
selections retain their original declaration whitelist.

This capability concerns the context of a nominal API, not the semantics of every
constant value. Constant references still require the existing compiler-resolved
same-unit primitive value contract; aggregate/generic/trait/nonlocal constant uses
remain outside that contract. An unused aggregate constant declaration can coexist
with a supported getter without making aggregate construction executable graph
support. Constants do not introduce a runtime global location or read operation.

The existing source visitor still descends through every initializer. Nested trait
impls, unsafe items and macro invocations cannot hide inside constant blocks.
Constant and associated-constant attributes pass the existing attribute boundary;
cfg/expansion attributes remain unavailable here. Trait impls, derives, item/body
macros, unsupported module selection and other closed-unit exclusions remain.
The nominal declaration, field tree, source ownership and drop checks are unchanged.

The capability is selected from the function's registered extraction profile.
The existing immutable source-environment lookup distinguishes this capability
alongside census ownership and verification policy, so a permissive check cannot
satisfy a legacy selection. No new serialized evidence, provenance record, source
snapshot or cache is added. A unit still parses/checks its shared source environment
once per supported policy/context; functions reuse that environment as before.

The real Cargo fixture composes module paths within one unit,
imports/aliases, inherent associated and module constants, field reads, constant
range patterns and a verified inherent helper caller. All admitted graph pairs are
loaded from the serialized unit and independently checked with their dependencies.
The earlier Data-v2 selection excludes the same nominal functions. A second Cargo
fixture verifies that an initializer containing a nested Drop impl still prevents
nominal admission while an independent scalar function remains admitted. Direct
census tests cover aggregate declarations, conditional attributes, macros and
hidden unsafe items; environment tests check capability-specific reuse.

S09 remains Partial: this removes the constant-declaration blocker without lifting
the general mixed-content, trait/expansion, cfg/module or ownership boundaries.
