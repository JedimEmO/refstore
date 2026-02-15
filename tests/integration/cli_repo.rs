use predicates::prelude::*;
use std::fs;

use crate::common::TestEnv;

#[test]
fn repo_add_local_dir() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.cmd()
        .args(["store", "add", "my-ref"])
        .arg(&sample)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'my-ref'"));

    // Content should be cached
    let cached = env.data_dir.path().join("content/my-ref");
    assert!(cached.exists(), "content dir should be created");
    assert!(
        cached.join("README.md").exists(),
        "README.md should be cached"
    );
    assert!(
        cached.join("src/lib.rs").exists(),
        "src/lib.rs should be cached"
    );
}

#[test]
fn repo_add_local_file() {
    let env = TestEnv::new();
    let file = env.create_sample_file();

    env.cmd()
        .args(["store", "add", "single-file"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'single-file'"));

    let cached = env.data_dir.path().join("content/single-file");
    assert!(cached.exists());
}

#[test]
fn repo_add_with_tags() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.cmd()
        .args(["store", "add", "tagged-ref"])
        .arg(&sample)
        .args(["--tag", "rust", "--tag", "example"])
        .assert()
        .success();

    // Tags should appear in list output
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("example"));
}

#[test]
fn repo_add_with_description() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.cmd()
        .args(["store", "add", "described-ref"])
        .arg(&sample)
        .args(["--description", "A useful reference"])
        .assert()
        .success();

    env.cmd()
        .args(["info", "described-ref"])
        .assert()
        .success()
        .stdout(predicate::str::contains("A useful reference"));
}

#[test]
fn repo_add_duplicate_fails() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("dup-ref", &sample);

    env.cmd()
        .args(["store", "add", "dup-ref"])
        .arg(&sample)
        .assert()
        .failure();
}

#[test]
fn repo_add_invalid_name() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.cmd()
        .args(["store", "add", "invalid name!"])
        .arg(&sample)
        .assert()
        .failure();
}

#[test]
fn repo_list_empty() {
    let env = TestEnv::new();

    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No references"));
}

#[test]
fn repo_list_filter_by_tag() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref_with_meta("ref-a", &sample, "First", &["rust"]);
    env.add_repo_ref_with_meta("ref-b", &sample, "Second", &["python"]);

    env.cmd()
        .args(["list", "--tag", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-a"))
        .stdout(predicate::str::contains("ref-b").not());
}

#[test]
fn repo_list_filter_by_kind() {
    let env = TestEnv::new();
    let sample_dir = env.create_sample_files();
    let sample_file = env.create_sample_file();

    env.add_repo_ref("dir-ref", &sample_dir);
    env.add_repo_ref("file-ref", &sample_file);

    env.cmd()
        .args(["list", "--kind", "file"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file-ref"))
        .stdout(predicate::str::contains("dir-ref").not());
}

#[test]
fn repo_info_shows_details() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref_with_meta("detailed-ref", &sample, "Detailed desc", &["tag1"]);

    env.cmd()
        .args(["info", "detailed-ref"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:"))
        .stdout(predicate::str::contains("detailed-ref"))
        .stdout(predicate::str::contains("Kind:"))
        .stdout(predicate::str::contains("Detailed desc"))
        .stdout(predicate::str::contains("tag1"));
}

#[test]
fn repo_info_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["info", "nonexistent"])
        .assert()
        .failure();
}

#[test]
fn repo_remove_force() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("to-remove", &sample);

    env.cmd()
        .args(["store", "remove", "--force", "to-remove"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 'to-remove'"));

    // Should no longer appear in list
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No references"));

    // Content should be cleaned up
    let cached = env.data_dir.path().join("content/to-remove");
    assert!(!cached.exists(), "content dir should be deleted");
}

#[test]
fn repo_remove_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["store", "remove", "--force", "ghost"])
        .assert()
        .failure();
}

#[test]
fn repo_update_refetches() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("updatable", &sample);

    // Modify the source
    fs::write(sample.join("README.md"), "# Updated\n").unwrap();

    env.cmd()
        .args(["store", "update", "updatable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    // Check the cached content reflects the update
    let cached = env.data_dir.path().join("content/updatable/README.md");
    let content = fs::read_to_string(cached).unwrap();
    assert_eq!(content, "# Updated\n");
}
