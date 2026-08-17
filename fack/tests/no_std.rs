use std::{path::PathBuf, process::Command};

#[test]
fn generated_errors_compile_in_a_no_std_consumer() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = manifest_dir.join("tests/no_std/Cargo.toml");
    let target = manifest_dir.join("../target/fack-no-std");
    let output = Command::new(env!("CARGO"))
        .args([
            "check",
            "--quiet",
            "--manifest-path",
            manifest.to_str().expect("fixture manifest path is UTF-8"),
        ])
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("cargo must execute for no_std fixture");

    assert!(
        output.status.success(),
        "no_std fixture failed\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
