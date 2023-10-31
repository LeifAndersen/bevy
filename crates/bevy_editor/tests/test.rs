use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::path::PathBuf;

#[test]
fn test_use_template() {
    let template_dir = PathBuf::from("assets/tests/simple.zip");
    let tempdir = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("init")
        .arg(tempdir.path())
        .env(bevy_editor::BEVY_TEMPLATE_DIR, template_dir)
        .assert();
    assert.success();
    tempdir.assert(predicate::path::is_dir());
    tempdir.child(".github").assert(predicate::path::is_dir());
    tempdir
        .child("LICENSE-Apache-2.0")
        .assert(predicate::path::is_file());
    tempdir
        .child("Cargo.toml")
        .assert(predicate::path::is_file());
}
