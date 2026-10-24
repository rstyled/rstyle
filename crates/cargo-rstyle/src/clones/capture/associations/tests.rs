use super::*;
#[path = "fixture.rs"]
mod fixture;
use fixture::{Fixture, native_metadata};

#[test]
fn import_only_dependency_seals_the_exact_artifact_without_direct_calls() {
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    fixture.functions[0][0].body.res = None;
    let edges = fixture.run().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["extern_alias"], "renamed");
    assert_eq!(
        edges[0]["artifact"]["checksum"],
        rstyle_pdg::unit_store::transport_checksum(&native_metadata())
    );
}

#[test]
fn anonymous_imports_and_calls_share_one_checked_artifact_edge() {
    let mut fixture = Fixture::new();
    let mut first = fixture.namespace_import();
    first.site.binding = namespaces::Binding::Anonymous;
    let mut second = first.clone();
    second.site.definition.definition_identity = "second-import-id".into();
    fixture.install_namespaces(vec![first, second]);
    assert_eq!(fixture.run().unwrap().len(), 1);
    fixture.functions[0][0].body.res = None;
    assert_eq!(fixture.run().unwrap().len(), 1);
}

#[test]
fn namespace_and_call_records_cannot_disagree_on_one_edge() {
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    fixture.change_foreign(|resolution| {
        resolution["compiled_dependency"]["crate_hash"] = json!("conflicting-hash");
    });
    assert!(
        fixture
            .run()
            .unwrap_err()
            .contains("Conflicting compiled dependency")
    );
}

#[test]
fn namespace_artifacts_reject_wrong_identity_alias_path_and_missing_owner() {
    for (field, value) in [
        ("identity", json!("wrong")),
        ("stable_crate_id", json!("wrong")),
        ("crate_name", json!("wrong")),
        ("rmeta", json!("/wrong.rmeta")),
    ] {
        let mut fixture = Fixture::new();
        let mut import = fixture.namespace_import();
        import.compiled_dependency[field] = value;
        fixture.install_namespaces(vec![import]);
        fixture.functions[0][0].body.res = None;
        assert!(fixture.run().is_err(), "{field}");
    }
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    fixture.functions[0][0].body.res = None;
    fixture.graph["units"][0]["dependencies"][0]["extern_crate_name"] = json!("wrong");
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    fixture.functions[0][0].body.res = None;
    fixture.functions[1].clear();
    assert!(fixture.run().is_err());
}

#[test]
fn malformed_unknown_and_repeated_unit_namespace_records_reject() {
    let mut fixture = Fixture::new();
    fixture.functions[0][0]
        .build
        .frontend_options
        .insert(namespaces::KEY.into(), json!({"malformed":true}));
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    let options = &mut fixture.functions[0][0].build.frontend_options;
    let mut changed = options.get(namespaces::KEY).unwrap().clone();
    changed["identity"] = json!("wrong");
    options.insert(namespaces::KEY.into(), changed);
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.install_namespaces(vec![fixture.namespace_import()]);
    let repeated = fixture.functions[0][0].clone();
    fixture.functions[0].push(repeated);
    assert!(fixture.run().is_err());
}

#[test]
fn repeated_namespace_imports_cannot_hide_conflicting_artifact_records() {
    let mut fixture = Fixture::new();
    let first = fixture.namespace_import();
    let mut second = first.clone();
    second.compiled_dependency["crate_hash"] = json!("conflicting-hash");
    fixture.install_namespaces(vec![first, second]);
    fixture.functions[0][0].body.res = None;
    assert!(
        fixture
            .run()
            .unwrap_err()
            .contains("Conflicting compiled dependency")
    );
}

#[test]
fn unselected_namespace_owner_remains_unassociated() {
    let mut fixture = Fixture::new();
    let mut import = fixture.namespace_import();
    import.site.target.crate_name = "unselected".into();
    import.compiled_dependency["crate_name"] = json!("unselected");
    fixture.install_namespaces(vec![import]);
    fixture.functions[0][0].body.res = None;
    assert!(fixture.run().unwrap().is_empty());
}

#[test]
fn incomplete_namespace_export_observations_still_check_artifact_context() {
    let mut fixture = Fixture::new();
    let mut import = fixture.namespace_import();
    import.complete = false;
    fixture.install_namespaces(vec![import]);
    fixture.functions[0][0].body.res = None;
    assert_eq!(fixture.run().unwrap().len(), 1);
    fixture.graph["units"][0]["dependencies"][0]["extern_crate_name"] = json!("wrong");
    assert!(fixture.run().is_err());
}

#[test]
fn exact_renamed_dependency_seals_emitted_bytes_once_per_edge() {
    let mut fixture = Fixture::new();
    let repeated = fixture.functions[0][0].clone();
    fixture.functions[0].push(repeated);
    let edges = fixture.run().unwrap();
    assert_eq!(edges.len(), 1);
    let edge = &edges[0];
    assert_eq!(edge["extern_alias"], "renamed");
    assert_eq!(edge["caller"]["unit"], 0);
    assert_eq!(edge["callee"]["graph_unit"], 1);
    assert_eq!(edge["artifact"]["bytes"], 64);
    assert_eq!(
        edge["artifact"]["checksum"],
        rstyle_pdg::unit_store::transport_checksum(&native_metadata())
    );
    assert_eq!(
        edge["local_compiled_crate"]["stable_crate_id"],
        "stable-calendar"
    );
}

#[test]
fn unselected_foreign_calls_remain_unassociated() {
    let mut fixture = Fixture::new();
    fixture.change_foreign(|resolution| {
        resolution["crate"] = json!("unselected");
        resolution
            .as_object_mut()
            .unwrap()
            .remove("compiled_dependency");
    });
    assert!(fixture.run().unwrap().is_empty());
}

#[test]
fn selected_missing_or_wrong_compiler_identity_rejects() {
    for (field, value) in [
        ("stable_crate_id", json!("wrong")),
        ("crate_name", json!("wrong")),
        ("identity", json!("unknown")),
        ("crate_hash", Value::Null),
        (
            "instance",
            json!({"type_arguments":["u8"],"const_arguments":[]}),
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.change_foreign(|resolution| resolution["compiled_dependency"][field] = value);
        assert!(fixture.run().is_err(), "{field}");
    }
    let mut fixture = Fixture::new();
    fixture.change_foreign(|resolution| {
        resolution
            .as_object_mut()
            .unwrap()
            .remove("compiled_dependency");
    });
    assert!(fixture.run().is_err());
}

#[test]
fn wrong_loaded_path_extern_alias_and_emitted_filename_reject() {
    let mut fixture = Fixture::new();
    fixture.change_foreign(|resolution| {
        resolution["compiled_dependency"]["rmeta"] = json!("/wrong.rmeta")
    });
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.graph["units"][0]["dependencies"][0]["extern_crate_name"] = json!("wrong_alias");
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.cargo[1]["filenames"] = json!(["/wrong.rmeta"]);
    assert!(fixture.run().is_err());
}

#[test]
fn missing_metadata_and_wrong_ledger_output_reject() {
    let mut fixture = Fixture::new();
    let loaded = fixture.directory.path().join("libcalendar-selected.rmeta");
    std::fs::remove_file(loaded).unwrap();
    assert!(fixture.run().is_err());
    fixture.units[1]
        .invocation
        .push("-Cextra-filename=-wrong".into());
    assert!(fixture.run().is_err());
}

#[test]
fn graph_artifact_package_context_and_ambiguous_edges_reject() {
    for field in ["package_id", "features", "profile"] {
        let mut fixture = Fixture::new();
        fixture.cargo[1][field] = Value::Null;
        assert!(fixture.run().is_err(), "{field}");
    }
    let mut fixture = Fixture::new();
    let duplicate = fixture.graph["units"][0]["dependencies"][0].clone();
    fixture.graph["units"][0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.graph["units"][0]["dependencies"][0]["index"] = json!(99);
    assert!(fixture.run().is_err());
}

#[test]
fn foreign_nodes_cannot_inherit_missing_callee_observations() {
    let mut fixture = Fixture::new();
    fixture.functions[1].clear();
    assert!(fixture.run().is_err());
}

#[test]
fn duplicate_compiler_artifacts_and_mismatched_invocations_reject() {
    let mut fixture = Fixture::new();
    fixture.cargo.push(fixture.cargo[1].clone());
    assert!(fixture.run().is_err());
    let mut fixture = Fixture::new();
    fixture.units[0]
        .invocation
        .push("--cfg=feature=\"unexpected\"".into());
    assert!(fixture.run().is_err());
}

#[test]
fn one_artifact_seal_is_reused_without_a_second_read() {
    let fixture = Fixture::new();
    let path = fixture.directory.path().join("libcalendar-selected.rmeta");
    let mut seals = Seals::new();
    let first = seal(&path, &mut seals).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(seal(&path, &mut seals).unwrap(), first);
    assert_eq!(seals.len(), 1);
}

#[test]
fn repeated_callsite_cannot_reuse_a_conflicting_compiler_record() {
    let mut fixture = Fixture::new();
    let original = fixture.functions[0][0].clone();
    fixture.change_foreign(|resolution| {
        resolution["compiled_dependency"]["crate_hash"] = json!("conflicting-hash");
    });
    let changed = fixture.functions[0][0].clone();
    fixture.functions[0][0] = original;
    fixture.functions[0].push(changed);
    assert!(fixture.run().is_err());
}

#[test]
fn disconnected_artifacts_lack_root_edge_authority() {
    let mut fixture = Fixture::new();
    fixture.graph["units"][0]["dependencies"] = json!([]);
    assert!(fixture.run().is_err());
}

#[test]
fn bounded_driver_sysroot_suffix_preserves_exact_ledger_prefix() {
    let mut fixture = Fixture::new();
    for functions in &mut fixture.functions {
        let options = &mut functions[0].build.frontend_options;
        let mut invocation = options.get("invocation").unwrap().clone();
        invocation
            .as_array_mut()
            .unwrap()
            .extend([json!("--sysroot"), json!("/actual/sysroot")]);
        options.insert("invocation".into(), invocation);
    }
    assert_eq!(fixture.run().unwrap().len(), 1);
}

#[test]
fn altered_prefix_and_unbounded_driver_suffixes_reject() {
    let expected = vec!["rustc".into(), "--crate-name".into(), "calendar".into()];
    for observed in [
        json!(["different-rustc", "--crate-name", "calendar"]),
        json!(["rustc", "--crate-name", "other"]),
        json!(["rustc", "--crate-name", "calendar", "--cfg", "changed"]),
        json!(["rustc", "--crate-name", "calendar", "--sysroot", ""]),
        json!([
            "rustc",
            "--crate-name",
            "calendar",
            "--sysroot",
            "/ok",
            "extra"
        ]),
        json!(["rustc", "--crate-name", "calendar", "--sysroot"]),
    ] {
        assert!(!invocation_matches(Some(&observed), &expected));
    }
}

#[test]
fn fixed_native_metadata_hash_uses_both_little_endian_fingerprint_halves() {
    assert_eq!(
        native_crate_hash(&native_metadata()).unwrap(),
        "0123456789abcdeffedcba9876543210"
    );
}

#[test]
fn changed_foreign_hash_rejects_even_with_same_stable_id_and_path() {
    let mut fixture = Fixture::new();
    fixture.change_foreign(|resolution| {
        resolution["compiled_dependency"]["crate_hash"] = json!("1123456789abcdeffedcba9876543210");
    });
    assert!(fixture.run().is_err());
}

#[test]
fn wrong_native_hash_rejects_after_transport_checksum_is_recomputed() {
    let fixture = Fixture::new();
    let path = fixture.directory.path().join("libcalendar-selected.rmeta");
    let original = seal(&path, &mut Seals::new()).unwrap();
    let mut changed = native_metadata();
    changed[24] ^= 1;
    std::fs::write(&path, changed).unwrap();
    let resealed = seal(&path, &mut Seals::new()).unwrap();
    assert_ne!(resealed.checksum, original.checksum);
    assert_ne!(resealed.crate_hash, original.crate_hash);
    assert!(fixture.run().is_err());
}

#[test]
fn malformed_native_headers_and_invalid_positions_reject() {
    for length in [0, 7, 23, 39] {
        assert!(native_crate_hash(&native_metadata()[..length]).is_err());
    }
    for offset in [0, 7] {
        let mut changed = native_metadata();
        changed[offset] ^= 1;
        assert!(native_crate_hash(&changed).is_err());
    }
    for offset in [8, 16] {
        for position in [0u64, 39, 64, u64::MAX] {
            let mut changed = native_metadata();
            changed[offset..offset + 8].copy_from_slice(&position.to_le_bytes());
            assert!(native_crate_hash(&changed).is_err());
        }
    }
}

#[test]
fn unknown_native_metadata_compiler_is_not_inferred_from_header() {
    let mut fixture = Fixture::new();
    fixture.functions[1][0].build.rustc_commit.value = Some("different-compiler".into());
    assert!(fixture.run().is_err());
    fixture.functions[1][0].build.rustc_commit.value = Some(COMPILER_COMMIT.into());
    fixture.functions[1][0].build.rustc_commit.status = rstyle_pdg::FactStatus::Unknown;
    assert!(fixture.run().is_err());
}
