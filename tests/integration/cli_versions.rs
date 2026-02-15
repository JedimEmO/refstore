use predicates::prelude::*;

use crate::common::TestEnv;

#[test]
fn versions_shows_history() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("my-docs", &sample);

    env.cmd()
        .args(["versions", "my-docs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version history for 'my-docs'"))
        .stdout(predicate::str::contains("Add reference: my-docs"));
}

#[test]
fn versions_shows_updates() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("my-docs", &sample);

    // Modify the source files so update produces a real content change
    std::fs::write(sample.join("README.md"), "# Updated\n").unwrap();

    // Update the reference
    env.cmd()
        .args(["store", "update", "my-docs"])
        .assert()
        .success();

    // Should show both the add and update commits
    env.cmd()
        .args(["versions", "my-docs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Add reference: my-docs"))
        .stdout(predicate::str::contains("Update reference: my-docs"));
}

#[test]
fn versions_unknown_ref_fails() {
    let env = TestEnv::new();

    env.cmd()
        .args(["versions", "nonexistent"])
        .assert()
        .failure();
}

#[test]
fn sync_with_version_pin() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    // Add a reference
    env.add_repo_ref("my-docs", &sample);

    // Create a tag on the registry
    std::process::Command::new("git")
        .args(["tag", "v1.0"])
        .current_dir(env.data_dir.path())
        .output()
        .unwrap();

    // Update the reference (creates new content at HEAD, different from v1.0)
    // First modify the source files
    std::fs::write(
        env.project_dir.path().join("sample/README.md"),
        "# Updated Sample\n",
    )
    .unwrap();
    env.cmd()
        .args(["store", "update", "my-docs"])
        .assert()
        .success();

    // Init project and add with version pin to v1.0
    env.init_project();
    env.cmd()
        .args(["add", "my-docs", "--pin", "v1.0"])
        .assert()
        .success();

    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("version: v1.0"));

    // The synced content should be from v1.0 (original content)
    let synced = env
        .project_dir
        .path()
        .join(".references/my-docs/README.md");
    let content = std::fs::read_to_string(synced).unwrap();
    assert_eq!(content, "# Sample Reference\n", "should contain v1.0 content, not updated content");
}

#[test]
fn sync_with_invalid_version_fails() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("my-docs", &sample);

    // Init project, add with bogus version pin
    env.init_project();
    env.cmd()
        .args(["add", "my-docs", "--pin", "nonexistent-tag"])
        .assert()
        .success();

    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("FAILED").or(predicate::str::contains("0 synced")))
        // It should report a failure for the version not found
        ;
}

#[test]
fn sync_without_version_uses_head() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("my-docs", &sample);

    // Create tag, then update
    std::process::Command::new("git")
        .args(["tag", "v1.0"])
        .current_dir(env.data_dir.path())
        .output()
        .unwrap();

    std::fs::write(
        env.project_dir.path().join("sample/README.md"),
        "# Updated Sample\n",
    )
    .unwrap();
    env.cmd()
        .args(["store", "update", "my-docs"])
        .assert()
        .success();

    // Init project, add WITHOUT version pin
    env.init_project();
    env.cmd()
        .args(["add", "my-docs"])
        .assert()
        .success();

    env.cmd().args(["sync"]).assert().success();

    // Should have the updated (HEAD) content
    let synced = env
        .project_dir
        .path()
        .join(".references/my-docs/README.md");
    let content = std::fs::read_to_string(synced).unwrap();
    assert_eq!(content, "# Updated Sample\n", "should contain HEAD content");
}

#[test]
fn repo_tag_create_and_list() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    env.add_repo_ref("my-docs", &sample);

    // Create a tag
    env.cmd()
        .args(["store", "tag", "v1.0", "-m", "First release"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created tag 'v1.0'"));

    // List tags
    env.cmd()
        .args(["store", "tags"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v1.0"));
}

#[test]
fn repo_tags_empty() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    env.add_repo_ref("my-docs", &sample);

    env.cmd()
        .args(["store", "tags"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tags"));
}
