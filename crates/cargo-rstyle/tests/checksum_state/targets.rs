use super::*;

#[test]
fn scalar_from_and_inline_owners_retain_ci_target_authorities() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            check_target(target, no_std);
        }
    }
}

fn check_target(target: &str, no_std: bool) {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        if no_std { "#![no_std]" } else { "" },
        r#"
        #[derive(Copy, Clone)] pub struct State { value: u16 }
        impl State {
            #[inline] pub fn new(value: u16) -> Self { Self { value } }
            #[inline(always)] pub fn value(&self) -> u32 { u32::from(self.value) }
        }
        #[inline(never)] pub fn wrapper(value: u16) -> u32 { State::new(value).value() }
        pub fn qualified(value: u16) -> u32 { core::primitive::u32::from(value) }
        type Word = u32;
        pub fn alias(value: u16) -> u32 { Word::from(value) }
        pub fn boolean(value: bool) -> u32 { u32::from(value) }
        pub fn partial(stop: bool, value: u16) -> u32 {
            u32::from({ if stop { return 77; } value })
        }
        pub fn saved(mut value: u16) -> u32 { let old = u32::from(value); value = 0; old | value as u32 }
    "#
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--target", target, "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{target}: {report}");
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let excluded = name.starts_with('<') || matches!(name, "boolean" | "alias");
        if name == "boolean" {
            let call = &function["facts"]["body"]["children"][0];
            let callee: Value =
                serde_json::from_str(call["res"]["description"].as_str().unwrap()).unwrap();
            assert!(
                callee["implementation"].is_null(),
                "{target}: unsupported From capture"
            );
        }
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{target}: {function}"
        );
    }
    check_aggregate_documents(&report);
}
