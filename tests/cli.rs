use std::{path::PathBuf, process::Command};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rust-experiment"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

#[test]
fn sample_csvs_produce_the_expected_accounts_without_logging() {
    for name in [
        "cli",
        "basic",
        "resolve",
        "chargeback",
        "multiple_disputes",
        "spent_dispute",
        "spent_chargeback",
        "wrong_references",
        "deposits_only",
        "invalid_rows",
        "overflow",
        "precision_boundary",
        "id_boundaries",
        "zero_and_negative",
        "header_only",
        "all_invalid",
        "invalid_utf8",
        "bom_crlf",
    ] {
        let output = binary()
            .arg(fixture(&format!("{name}.csv")))
            .output()
            .unwrap();
        assert!(output.status.success(), "{name}: {:?}", output.stderr);
        assert!(output.stderr.is_empty(), "{name}: unexpected stderr");
        let expected = std::fs::read(fixture(&format!("{name}.expected.csv"))).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(expected).unwrap(),
            "sample: {name}"
        );
    }
}

#[test]
fn invalid_headers_and_empty_input_fail_silently() {
    for name in [
        "empty.csv",
        "bad_header.csv",
        "reordered_header.csv",
        "headerless.csv",
    ] {
        let output = binary().arg(fixture(name)).output().unwrap();
        assert!(!output.status.success(), "{name}");
        assert!(output.stdout.is_empty(), "{name}");
        assert!(output.stderr.is_empty(), "{name}");
    }
}

#[test]
fn requires_exactly_one_input_path_without_printing_usage() {
    for arguments in [vec![], vec!["first.csv", "second.csv"]] {
        let output = binary().args(arguments).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn missing_file_fails_silently() {
    let path = fixture("file-that-does-not-exist.csv");
    assert!(!path.exists());
    let output = binary().arg(path).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn stdout_write_failure_exits_unsuccessfully_without_logging() {
    let sink = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = binary()
        .arg(fixture("cli.csv"))
        .stdout(sink)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
}
