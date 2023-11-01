use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::path::PathBuf;

const ASSETS_DIR: &str = "assets/tests";

#[test]
fn test_use_zip_template() {
    let template_dir = PathBuf::from(format!("{}/templates", ASSETS_DIR));
    let tempdir = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("init")
        .arg(tempdir.path())
        .arg("-t")
        .arg("simple")
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
    tempdir
        .child("src/simple.rs")
        .assert(predicate::path::is_file());
    tempdir
        .child("src/simple_raw.rs")
        .assert(predicate::path::is_file());
    tempdir
        .child("src/simple.rs.tera")
        .assert(predicate::path::missing());
}

#[test]
fn test_use_git_template() {}
