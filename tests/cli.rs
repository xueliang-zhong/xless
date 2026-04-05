use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn dumps_effective_config() {
    let mut cmd = Command::cargo_bin("xless").unwrap();
    cmd.arg("--dump-config")
        .arg("--line-numbers")
        .assert()
        .success()
        .stdout(contains("line_numbers = true"));
}
