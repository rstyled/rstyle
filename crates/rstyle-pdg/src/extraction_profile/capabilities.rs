//! Capability IDs follow the historical, append-only registration order.
//! IDs are internal; evidence continues to use the original capability names.

macro_rules! capabilities {
    ($($id:ident => $name:literal,)*) => {
        #[derive(Clone, Copy)]
        #[repr(u8)]
        pub(super) enum CapabilityId {
            $($id,)*
        }

        impl CapabilityId {
            pub(super) fn parse(name: &str) -> Option<Self> {
                const BY_NAME: &[(&str, CapabilityId)] = &sort_names([$(($name, CapabilityId::$id),)*]);
                BY_NAME
                    .binary_search_by(|(registered, _)| registered.cmp(&name))
                    .ok()
                    .map(|index| BY_NAME[index].1)
            }

            pub(super) fn enabled(self, count: usize) -> bool {
                (self as usize) < count
            }
        }

        pub(super) const EXPANDED_CAPABILITIES: &[&str] = &[$($name,)*];
    };
}

// Build the name index at compile time; ordinal IDs retain registration order.
const fn sort_names<const N: usize>(
    mut names: [(&str, CapabilityId); N],
) -> [(&str, CapabilityId); N] {
    let mut index = 1;
    while index < N {
        let entry = names[index];
        let mut position = index;
        while position > 0 && name_before(entry.0, names[position - 1].0) {
            names[position] = names[position - 1];
            position -= 1;
        }
        names[position] = entry;
        index += 1;
    }
    names
}

const fn name_before(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut index = 0;
    while index < left.len() && index < right.len() {
        if left[index] != right[index] {
            return left[index] < right[index];
        }
        index += 1;
    }

    left.len() < right.len()
}

capabilities! {
    ConstFunctionBodies => "rust.const-function-bodies",
    IntegerCasts => "rust.integer-casts",
    ScalarMatch => "rust.scalar-match",
    ScalarMatchGuards => "rust.scalar-match-guards",
    ConditionalSourceCorrespondence => "rust.conditional-source-correspondence",
    ScalarLoops => "rust.scalar-loops",
    Divergence => "rust.divergence",
    VerifiedPrimitiveSummaries => "rust.verified-primitive-summaries",
    VerifiedHelperCalls => "rust.verified-helper-calls",
    ConcreteReadOnlyData => "rust.concrete-read-only-data",
    VerifiedReadOnlyHelperCalls => "rust.verified-read-only-helper-calls",
    VerifiedInherentHelperCalls => "rust.verified-inherent-helper-calls",
    ScalarOrPatterns => "rust.scalar-or-patterns",
    ScalarRangePatterns => "rust.scalar-range-patterns",
    ResolvedScalarConstants => "rust.resolved-scalar-constants",
    ScalarConstantPatterns => "rust.scalar-constant-patterns",
    ScalarConstantRangeBounds => "rust.scalar-constant-range-bounds",
    ScalarAssertions => "rust.scalar-assertions",
    CfgSourceRemoval => "rust.cfg-source-removal",
    CfgInactiveDefinitions => "rust.cfg-inactive-definitions",
    ExhaustiveScalarMatch => "rust.exhaustive-scalar-match",
    NestedScalarOrPatterns => "rust.nested-scalar-or-patterns",
    NominalConstantContext => "rust.nominal-constant-context",
    AggregateLiteralValues => "rust.aggregate-literal-values",
    RepeatArrayValues => "rust.repeat-array-values",
    NamedStructValues => "rust.named-struct-values",
    StructUpdateValues => "rust.struct-update-values",
    TupleStructValues => "rust.tuple-struct-values",
    UnitStructValues => "rust.unit-struct-values",
    LocalAggregateValues => "rust.local-aggregate-values",
    AggregateValueFields => "rust.aggregate-value-fields",
    SharedReferenceLocals => "rust.shared-reference-locals",
    SharedLocalFieldReads => "rust.shared-local-field-reads",
    ReferenceBearingValues => "rust.reference-bearing-values",
    ReferenceValueFields => "rust.reference-value-fields",
    ReferenceStructUpdates => "rust.reference-struct-updates",
    BodyNominalTypes => "rust.body-nominal-types",
    StaticInherentHelperCalls => "rust.static-inherent-helper-calls",
    ReferenceRebindings => "rust.reference-rebindings",
    IndirectFieldReads => "rust.indirect-field-reads",
    SharedDereferenceReads => "rust.shared-dereference-reads",
    ScalarIndexReads => "rust.scalar-index-reads",
    SharedSubsliceViews => "rust.shared-subslice-views",
    SliceArrayByteCalls => "rust.slice-array-byte-calls",
    NativeBorrowedByteReaders => "rust.native-borrowed-byte-readers",
    ScalarTupleAssignment => "rust.scalar-tuple-assignment",
    OutsideLoopValueMerges => "rust.outside-loop-value-merges",
    CheckedSliceOption => "rust.checked-slice-option",
    CopyNominalBoundary => "rust.copy-nominal-boundary",
    OwnedCopyReceiver => "rust.owned-copy-receiver",
    OwnedCopySelf => "rust.owned-copy-self",
    BorrowedScalarOwners => "rust.borrowed-scalar-owners",
    CyclicValueMerges => "rust.cyclic-value-merges",
    ScalarOptionValues => "rust.scalar-option-values",
    ScalarFromConversions => "rust.scalar-from-conversions",
    NativePrefixReaders => "rust.native-prefix-readers",
    ScopedNominalImports => "rust.scoped-nominal-imports",
    CopyValueHelperReads => "rust.copy-value-helper-reads",
    CopyArrayFields => "rust.copy-array-fields",
    UsizeFromU8 => "rust.usize-from-u8",
    BoundedUnsignedReads => "rust.bounded-unsigned-reads",
    ScalarEnumValues => "rust.scalar-enum-values",
    TaggedUnitClassifiers => "rust.tagged-unit-classifiers",
    ImmediateOptionCallbacks => "rust.immediate-option-callbacks",
    BorrowedArrayViews => "rust.borrowed-array-views",
    BitsetCardinality => "rust.bitset-cardinality",
    EndOfInput => "rust.end-of-input",
    AcceleratorPacking => "rust.accelerator-packing",
    ByteIteratorPosition => "rust.byte-iterator-position",
    ByteScalarLiterals => "rust.byte-scalar-literals",
    LocalConstBoolTables => "rust.local-const-bool-tables",
    NamedOptionCallbacks => "rust.named-option-callbacks",
    BorrowedCopyChildFields => "rust.borrowed-copy-child-fields",
    IndexedReadHelperCalls => "rust.indexed-read-helper-calls",
    FieldlessEnumValues => "rust.fieldless-enum-values",
    CharacterScalars => "rust.character-scalars",
    CharacterLiteralRanges => "rust.character-literal-ranges",
    CharacterAsciiPredicate => "rust.character-ascii-predicate",
    FloatKernelScalars => "rust.float-kernel-scalars",
    CfgIfModuleOwnership => "rust.cfg-if-module-ownership",
    ClosedScalarModuleDependencies => "rust.closed-scalar-module-dependencies",
    FloatSubtraction => "rust.float-subtraction",
    ClosedScalarControlDependencies => "rust.closed-scalar-control-dependencies",
    BorrowedMixedOwners => "rust.borrowed-mixed-owners",
    PassiveCopyDependencies => "rust.passive-copy-dependencies",
    StoredSliceHelperResults => "rust.stored-slice-helper-results",
    CheckedUsizeLengths => "rust.checked-usize-lengths",
    ScalarOptionIfLet => "rust.scalar-option-if-let",
    SharedHelperOptionCallbacks => "rust.shared-helper-option-callbacks",
    SaturatingUsizeLengths => "rust.saturating-usize-lengths",
    LiteralDocumentationInputs => "rust.literal-documentation-inputs",
    BoolToUsize => "rust.bool-to-usize",
    PinnedAllocMacroImports => "rust.pinned-alloc-macro-imports",
    FreshRangeResults => "rust.fresh-range-results",
    BoundedScalarLoopHelpers => "rust.bounded-scalar-loop-helpers",
    PinnedPrimitiveConstants => "rust.pinned-primitive-constants",
    FreshConcreteResults => "rust.fresh-concrete-results",
    SameErrorResultPropagation => "rust.same-error-result-propagation",
    ConcreteOptionPropagation => "rust.concrete-option-propagation",
    CheckedBorrowedResultFlow => "rust.checked-borrowed-result-flow",
    ExplicitU8FieldlessEnums => "rust.explicit-u8-fieldless-enums",
    VerifiedCrossUnitScalarHelpers => "rust.verified-cross-unit-scalar-helpers",
    CheckedScalarInstances => "rust.checked-scalar-instances",
    ComponentHelperResults => "rust.component-helper-results",
    CheckedReferenceConstructorInstances => "rust.checked-reference-constructor-instances",
    CheckedStringByteViewInstances => "rust.checked-string-byte-view-instances",
    CheckedReferenceForwarderInstances => "rust.checked-reference-forwarder-instances",
    CopyFieldlessResultFlow => "rust.copy-fieldless-result-flow",
    ScalarSuccessResultPropagation => "rust.scalar-success-result-propagation",
    CheckedSuffixResultViews => "rust.checked-suffix-result-views",
    FreshReferenceProductResults => "rust.fresh-reference-product-results",
    LinearProductResultFlow => "rust.linear-product-result-flow",
    CheckedProductConstructorInstances => "rust.checked-product-constructor-instances",
    ConsumingRangeConversions => "rust.consuming-range-conversions",
    MutableScalarFormals => "rust.mutable-scalar-formals",
    CheckedConcreteDefault => "rust.checked-concrete-default",
    CheckedCopyOwnerSource => "rust.checked-copy-owner-source",
    LocalValueTraitCalls => "rust.local-value-trait-calls",
    LosslessUsizeU64Calls => "rust.lossless-usize-u64-calls",
    FreshScalarErrorResults => "rust.fresh-scalar-error-results",
    NominalProductConstants => "rust.nominal-product-constants",
    CheckedConcreteTryfrom => "rust.checked-concrete-tryfrom",
    ConstructorResultMaps => "rust.constructor-result-maps",
    CheckedAnonymousDependencyNamespaces => "rust.checked-anonymous-dependency-namespaces",
    StaticStringCopyProducts => "rust.static-string-copy-products",
    CheckedCopyConstructorSites => "rust.checked-copy-constructor-sites",
    CopyTaggedConversionValues => "rust.copy-tagged-conversion-values",
    CopyTaggedConversionHelperCalls => "rust.copy-tagged-conversion-helper-calls",
    SlicePrefixProductResults => "rust.slice-prefix-product-results",
    CheckedGeneratedDependencyNamespaces => "rust.checked-generated-dependency-namespaces",
    CheckedLocalMacroOwnership => "rust.checked-local-macro-ownership",
    CheckedExternalExpressionMacroOwnership => "rust.checked-external-expression-macro-ownership",
    CheckedDigitProductValues => "rust.checked-digit-product-values",
    WideningPreludeSource => "rust.widening-prelude-source",
    CheckedCorrectedExpressionMacroOwnership => "rust.checked-corrected-expression-macro-ownership",
    CheckedDigitProductSummaries => "rust.checked-digit-product-summaries",
    CheckedDigitProductForwarders => "rust.checked-digit-product-forwarders",
    ByteArrayCheckedResults => "rust.byte-array-checked-results",
    U32UsizeValueTraitCalls => "rust.u32-usize-value-trait-calls",
    LosslessU32UsizeCalls => "rust.lossless-u32-usize-calls",
    U64FromU32 => "rust.u64-from-u32",
    OrderedLocalMacroSource => "rust.ordered-local-macro-source",
    CheckedNonzeroDigitComposition => "rust.checked-nonzero-digit-composition",
    UnsignedIndexConversionFamily => "rust.unsigned-index-conversion-family",
    GeneratedCheckedTryfromMaps => "rust.generated-checked-tryfrom-maps",
    NamedErrorPayloadFlow => "rust.named-error-payload-flow",
    NamedErrorHelperCalls => "rust.named-error-helper-calls",
    PinnedCleanupMacroImports => "rust.pinned-cleanup-macro-imports",
    NamedErrorReaderFlow => "rust.named-error-reader-flow",
    NamedErrorValidatorFlow => "rust.named-error-validator-flow",
    NamedErrorProductReaderFlow => "rust.named-error-product-reader-flow",
    NamedErrorOwnedTransfer => "rust.named-error-owned-transfer",
    NamedErrorIdReaderFlow => "rust.named-error-id-reader-flow",
    UsizeReaderCallbackObservations => "rust.usize-reader-callback-observations",
    NamedErrorUsizeReaderFlow => "rust.named-error-usize-reader-flow",
    NamedErrorU16ReaderFlow => "rust.named-error-u16-reader-flow",
    U16UsizeReaderObservations => "rust.u16-usize-reader-observations",
    NamedErrorU16UsizeReaderFlow => "rust.named-error-u16-usize-reader-flow",
    FreshScalarOwners => "rust.fresh-scalar-owners",
}
