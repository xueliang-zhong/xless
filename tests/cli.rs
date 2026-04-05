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

#[test]
fn accepts_less_style_raw_control_alias() {
    let mut cmd = Command::cargo_bin("xless").unwrap();
    cmd.arg("--dump-config")
        .arg("-r")
        .assert()
        .success()
        .stdout(contains("raw_control_chars = true"));
}

#[test]
fn applies_less_style_tab_width_override() {
    let mut cmd = Command::cargo_bin("xless").unwrap();
    cmd.arg("--dump-config")
        .arg("-x")
        .arg("8")
        .assert()
        .success()
        .stdout(contains("tab_width = 8"));
}

#[test]
fn help_shows_the_raw_control_alias() {
    let mut cmd = Command::cargo_bin("xless").unwrap();
    cmd.arg("--help").assert().success().stdout(contains("-r"));
}
