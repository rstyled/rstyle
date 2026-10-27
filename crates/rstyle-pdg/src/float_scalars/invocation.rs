//! Float annotations require ordinary rustc codegen, without backend overrides.
use crate::unit_store::EvidenceFacts;

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let arguments = facts
        .build
        .frontend_options
        .get("invocation")
        .and_then(serde_json::Value::as_array)
        .ok_or("Float semantics require the observed compiler invocation")?;
    if arguments.iter().any(|value| !value.is_string()) {
        return Err("Float compiler invocation contains a non-string argument".into());
    }
    check(
        target(facts)?,
        arguments
            .iter()
            .map(|value| value.as_str().expect("checked string argument")),
    )
}

fn check<'a>(
    expected_target: &str,
    arguments: impl Iterator<Item = &'a str>,
) -> Result<(), String> {
    let mut arguments = nonempty(arguments)?;
    while let Some(argument) = arguments.next() {
        if argument.starts_with('@') || argument.starts_with("-Z") {
            return Err(
                "Float semantics do not support response files or unstable compiler overrides"
                    .into(),
            );
        }
        let selected_target = if argument == "--target" {
            Some(
                arguments
                    .next()
                    .ok_or("Missing float compiler target argument")?,
            )
        } else {
            argument.strip_prefix("--target=")
        };
        if selected_target.is_some_and(|selected| selected != expected_target) {
            return Err("Float semantics require the exact registered named target".into());
        }
        let option = if argument == "-C" || argument == "--codegen" {
            Some(
                arguments
                    .next()
                    .ok_or("Missing float compiler codegen argument")?,
            )
        } else {
            argument
                .strip_prefix("-C")
                .or_else(|| argument.strip_prefix("--codegen="))
        };
        if option.is_some_and(unsafe_codegen) {
            return Err(
                "Float semantics do not support custom floating-point/backend codegen overrides"
                    .into(),
            );
        }
    }
    Ok(())
}

fn nonempty<'a>(
    mut arguments: impl Iterator<Item = &'a str>,
) -> Result<impl Iterator<Item = &'a str>, String> {
    let first = arguments
        .next()
        .ok_or("Float semantics require a nonempty compiler invocation")?;
    Ok(std::iter::once(first).chain(arguments))
}

fn target(facts: &dyn EvidenceFacts) -> Result<&str, String> {
    let selected = facts
        .build
        .frontend_options
        .get("target")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing float selected target identity")?;
    let llvm = match selected {
        "aarch64-unknown-linux-gnu" | "x86_64-unknown-linux-gnu" => selected,
        "aarch64-apple-darwin" => "arm64-apple-macosx",
        _ => return Err("Float semantics do not support custom target specifications".into()),
    };
    if facts.build.target_triple.known_value() != Some(llvm) {
        return Err("Float named target contradicts its observed LLVM target".into());
    }
    Ok(selected)
}

fn unsafe_codegen(option: &str) -> bool {
    let key = option.split('=').next().unwrap_or_default();
    [
        "llvm-args",
        "passes",
        "target-feature",
        "target-cpu",
        "soft-float",
        "linker-plugin-lto",
    ]
    .iter()
    .any(|expected| {
        key.bytes()
            .map(|byte| if byte == b'_' { b'-' } else { byte })
            .eq(expected.bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn ordinary_cargo_debug_release_and_no_std_invocations_remain_supported() {
        for arguments in [
            vec![
                "rustc",
                "--crate-type",
                "lib",
                "-C",
                "debuginfo=2",
                "--cfg",
                "feature=std",
            ],
            vec![
                "rustc",
                "--crate-type=lib",
                "-Copt-level=3",
                "--codegen",
                "lto=thin",
            ],
            vec!["rustc", "--codegen=opt-level=s", "-C", "overflow-checks=on"],
        ] {
            check("aarch64-unknown-linux-gnu", arguments.iter().copied()).unwrap();
        }
    }

    #[test]
    fn backend_injection_cannot_claim_default_ieee_rounding_or_nan_semantics() {
        for argument in [
            "-Cllvm-args=-enable-unsafe-fp-math",
            "--codegen=passes=instcombine",
            "-Ctarget-feature=+fp64",
            "-Ctarget-cpu=native",
            "-Csoft-float=yes",
            "-Clinker-plugin-lto=yes",
            "-Zcodegen-backend=custom",
            "@hidden.rsp",
            "-Cllvm_args=-enable-unsafe-fp-math",
            "-Ctarget_feature=+fp64",
            "-Ctarget_cpu=native",
            "-Clinker_plugin_lto=yes",
            "--target=custom.json",
            "--target=x86_64-unknown-linux-gnu",
        ] {
            assert!(
                check("aarch64-unknown-linux-gnu", ["rustc", argument].into_iter()).is_err(),
                "{argument}"
            );
        }
    }

    #[test]
    fn split_options_and_named_targets_keep_the_same_boundaries() {
        for key in ["-C", "--codegen"] {
            assert!(
                check(
                    "aarch64-unknown-linux-gnu",
                    ["rustc", key, "llvm-args=-enable-no-nans-fp-math"].into_iter()
                )
                .is_err()
            );
            assert!(check("aarch64-unknown-linux-gnu", ["rustc", key].into_iter()).is_err());
        }
        assert!(
            check(
                "aarch64-unknown-linux-gnu",
                ["rustc", "-Z", "mir-opt-level=4"].into_iter()
            )
            .is_err()
        );
        assert!(check("aarch64-unknown-linux-gnu", [].into_iter()).is_err());
        assert!(
            check(
                "aarch64-unknown-linux-gnu",
                ["rustc", "--target", "custom.json"].into_iter()
            )
            .is_err()
        );
        check(
            "aarch64-unknown-linux-gnu",
            ["rustc", "--target", "aarch64-unknown-linux-gnu"].into_iter(),
        )
        .unwrap();
    }
}
