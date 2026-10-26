use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tokenradar"))
        .args(args)
        .output()
        .expect("failed to run the tokenradar binary")
}

#[test]
fn version_flag_prints_package_version() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            format!("tokenradar {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn unknown_flag_fails_with_usage_error() {
    let out = run(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--bogus"));
}
