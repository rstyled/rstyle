# Bounded concrete Option observations

Data-v3 captures the builtin core Option with a bool/integer scalar payload or
one shared reference to such a scalar. It uses the existing concrete-type
observation record and recursive budgets: `Option(Shared(Scalar("u8")))` describes
the selected slice-get result. This is an observational type, not an ABI layout,
discriminant encoding, supported interface or registered call contract.

The normalized vocabulary can encode the observation, but semantic
`data_types::parse` deliberately rejects Option until the connected checked-get,
variant guard and payload-load contract is registered. Construction, returns,
Option parameters, matching and extraction are not certified by this increment.
The extraction policy version/capabilities and matching profile stay unchanged.
Data-v1 and Data-v2 capture their historical records unchanged.

The driver recognizes the actual compiler diagnostic Option definition and its
resolved Some/None variants. The existing pattern resolution contains the core
crate/hash/definition identity. Renamed imports and type aliases retain those
identities. A user enum called Option cannot acquire the builtin marker through
spelling. Flat Some children and explicit immutable scalar-reference patterns
retain their original child bindings, HIR identities, types and source spans.
Other pattern children remain explicitly unsupported under their existing rules.
Mutable and nested Option payloads remain outside this bounded observation.

Owning nominal fields remain unsupported. Observing Option inside a struct would
otherwise expose that struct to source-proof code that supports only lifetime
arguments and cannot establish the Option field's original type. The driver
uses the existing in-progress nominal stack to retain the whole owning type as
unsupported, without dropping fields, weakening proof or adding another tree
walk. Standalone Option locals, parameters, results and method observations can
still be retained. A scalar leaf beside such a struct remains assessable.

This advances the source-selected regex-automata `util::utf8::is_boundary` chain:
its Some(&b) arm now retains b and its variant/parent relationship. It does not
model the byte read. Next connect core slice get authority and bounds, conditional
reference origins, executable Some/None discriminators and a byte load under the
Some guard, including saved-origin and helper-result composition. The complete
family must pass real Cargo capture, independent document verification, strict
admission and matching before D17 can be reported as supported.
