use predicates::prelude::*;
use std::fs;

use crate::common::TestEnv;

#[test]
fn init_creates_manifest() {
    let env = TestEnv::new();

    env.cmd()
        .args(["init", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized refstore"));

    assert!(env.project_dir.path().join("refstore.toml").exists());
    assert!(env.project_dir.path().join(".references").exists());
}

#[test]
fn init_adds_gitignore() {
    let env = TestEnv::new();

    env.init_project();

    let gitignore = env.project_dir.path().join(".gitignore");
    assert!(gitignore.exists(), ".gitignore should be created");
    let content = fs::read_to_string(gitignore).unwrap();
    assert!(
        content.contains(".references/"),
        ".gitignore should contain .references/"
    );
}

#[test]
fn init_commit_references() {
    let env = TestEnv::new();

    env.cmd()
        .args(["init", "--path"])
        .arg(env.project_dir.path())
        .arg("--commit-references")
        .assert()
        .success()
        .stdout(predicate::str::contains("committed to git"));

    // .gitignore should NOT exist (or not contain .references/)
    let gitignore = env.project_dir.path().join(".gitignore");
    if gitignore.exists() {
        let content = fs::read_to_string(gitignore).unwrap();
        assert!(
            !content.contains(".references/"),
            ".gitignore should not contain .references/"
        );
    }
}

#[test]
fn init_twice_fails() {
    let env = TestEnv::new();

    env.init_project();

    env.cmd()
        .args(["init", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .failure();
}

#[test]
fn add_reference_to_project() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("proj-ref", &sample);
    env.init_project();

    env.cmd()
        .args(["add", "proj-ref"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'proj-ref'"));

    // Manifest should contain the reference
    let manifest = fs::read_to_string(env.project_dir.path().join("refstore.toml")).unwrap();
    assert!(
        manifest.contains("proj-ref"),
        "manifest should contain proj-ref"
    );
}

#[test]
fn add_unknown_reference_fails() {
    let env = TestEnv::new();
    env.init_project();

    env.cmd()
        .args(["add", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn remove_reference_from_project() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("removable", &sample);
    env.init_project();

    env.cmd().args(["add", "removable"]).assert().success();

    env.cmd()
        .args(["remove", "removable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 'removable'"));

    let manifest = fs::read_to_string(env.project_dir.path().join("refstore.toml")).unwrap();
    assert!(
        !manifest.contains("removable"),
        "manifest should no longer contain removable"
    );
}

#[test]
fn remove_with_purge() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("purgeable", &sample);
    env.init_project();
    env.cmd().args(["add", "purgeable"]).assert().success();
    env.cmd().args(["sync"]).assert().success();

    let ref_dir = env.project_dir.path().join(".references/purgeable");
    assert!(ref_dir.exists(), "synced content should exist");

    env.cmd()
        .args(["remove", "purgeable", "--purge"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Purged"));

    assert!(!ref_dir.exists(), "purged content should be removed");
}

#[test]
fn sync_copies_content() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("syncable", &sample);
    env.init_project();
    env.cmd().args(["add", "syncable"]).assert().success();

    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("synced"));

    let ref_dir = env.project_dir.path().join(".references/syncable");
    assert!(ref_dir.exists());
    assert!(ref_dir.join("README.md").exists());
    assert!(ref_dir.join("src/lib.rs").exists());
    assert!(ref_dir.join("docs/guide.md").exists());
}

#[test]
fn sync_specific_reference() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-one", &sample);
    env.add_repo_ref("ref-two", &sample);
    env.init_project();
    env.cmd().args(["add", "ref-one"]).assert().success();
    env.cmd().args(["add", "ref-two"]).assert().success();

    // Only sync ref-one
    env.cmd()
        .args(["sync", "ref-one"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-one"));

    assert!(env
        .project_dir
        .path()
        .join(".references/ref-one")
        .exists());
    assert!(!env
        .project_dir
        .path()
        .join(".references/ref-two")
        .exists());
}

#[test]
fn sync_force_overwrites() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("forceable", &sample);
    env.init_project();
    env.cmd().args(["add", "forceable"]).assert().success();
    env.cmd().args(["sync"]).assert().success();

    // Modify the synced content
    let ref_file = env
        .project_dir
        .path()
        .join(".references/forceable/README.md");
    fs::write(&ref_file, "modified").unwrap();

    // Force sync should overwrite
    env.cmd().args(["sync", "--force"]).assert().success();

    let content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(content, "# Sample Reference\n");
}

#[test]
fn status_shows_synced() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("status-ref", &sample);
    env.init_project();
    env.cmd().args(["add", "status-ref"]).assert().success();
    env.cmd().args(["sync"]).assert().success();

    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status-ref"))
        .stdout(predicate::str::contains("synced"));
}

#[test]
fn status_shows_not_synced() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("unsync-ref", &sample);
    env.init_project();
    env.cmd().args(["add", "unsync-ref"]).assert().success();

    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not synced"));
}

#[test]
fn full_workflow() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    // 1. Add to central repo
    env.add_repo_ref_with_meta("workflow-ref", &sample, "End to end test", &["test"]);

    // 2. Verify it's in the repo
    env.cmd()
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("workflow-ref"));

    // 3. Init project
    env.init_project();

    // 4. Add to project
    env.cmd().args(["add", "workflow-ref"]).assert().success();

    // 5. Status should show not synced
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not synced"));

    // 6. Sync
    env.cmd().args(["sync"]).assert().success();

    // 7. Status should show synced
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("synced"))
        .stdout(predicate::str::contains("not synced").not());

    // 8. Content should exist
    assert!(env
        .project_dir
        .path()
        .join(".references/workflow-ref/README.md")
        .exists());

    // 9. Remove from project with purge
    env.cmd()
        .args(["remove", "workflow-ref", "--purge"])
        .assert()
        .success();

    assert!(!env
        .project_dir
        .path()
        .join(".references/workflow-ref")
        .exists());

    // 10. Clean up central repo
    env.cmd()
        .args(["repo", "remove", "--force", "workflow-ref"])
        .assert()
        .success();

    env.cmd()
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No references"));
}
