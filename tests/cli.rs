use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn dumps_effective_config() {
    let mut cmd = Command::cargo_bin("xless").unwrap();
    cmd.arg("--dump-config")
        .arg("--no-highlight")
        .arg("--line-numbers")
        .assert()
        .success()
        .stdout(contains("line_numbers = true"))
        .stdout(contains("highlight = false"));
}
