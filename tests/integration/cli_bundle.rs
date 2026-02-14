use predicates::prelude::*;
use std::fs;

use crate::common::TestEnv;

#[test]
fn bundle_create() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);

    env.cmd()
        .args(["repo", "bundle", "create", "my-stack", "--ref", "ref-a", "--ref", "ref-b"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created bundle 'my-stack'"));

    // Verify it shows in list
    env.cmd()
        .args(["repo", "bundle", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-stack"))
        .stdout(predicate::str::contains("2 refs"));
}

#[test]
fn bundle_create_duplicate_fails() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.create_bundle("dupe-bundle", &["ref-a"]);

    env.cmd()
        .args(["repo", "bundle", "create", "dupe-bundle", "--ref", "ref-a"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn bundle_create_invalid_ref_fails() {
    let env = TestEnv::new();

    env.cmd()
        .args(["repo", "bundle", "create", "bad-bundle", "--ref", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown reference"));
}

#[test]
fn bundle_list_empty() {
    let env = TestEnv::new();

    env.cmd()
        .args(["repo", "bundle", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles"));
}

#[test]
fn bundle_list_with_tag_filter() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);

    env.cmd()
        .args([
            "repo", "bundle", "create", "tagged-bundle",
            "--ref", "ref-a",
            "--tag", "rust",
        ])
        .assert()
        .success();

    env.cmd()
        .args([
            "repo", "bundle", "create", "untagged-bundle",
            "--ref", "ref-b",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["repo", "bundle", "list", "--tag", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tagged-bundle"))
        .stdout(predicate::str::contains("untagged-bundle").not());
}

#[test]
fn bundle_info() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);

    env.cmd()
        .args([
            "repo", "bundle", "create", "info-bundle",
            "--ref", "ref-a", "--ref", "ref-b",
            "--description", "Test bundle",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["repo", "bundle", "info", "info-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("info-bundle"))
        .stdout(predicate::str::contains("Test bundle"))
        .stdout(predicate::str::contains("ref-a"))
        .stdout(predicate::str::contains("ref-b"));
}

#[test]
fn bundle_update_add_remove_refs() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);
    env.add_repo_ref("ref-c", &sample);

    env.create_bundle("update-bundle", &["ref-a", "ref-b"]);

    // Add ref-c, remove ref-a
    env.cmd()
        .args([
            "repo", "bundle", "update", "update-bundle",
            "--add-ref", "ref-c",
            "--remove-ref", "ref-a",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated bundle"));

    // Verify: should have ref-b and ref-c, not ref-a
    env.cmd()
        .args(["repo", "bundle", "info", "update-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-b"))
        .stdout(predicate::str::contains("ref-c"))
        .stdout(predicate::str::contains("ref-a").not());
}

#[test]
fn bundle_remove() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.create_bundle("removable-bundle", &["ref-a"]);

    env.cmd()
        .args(["repo", "bundle", "remove", "--force", "removable-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed bundle"));

    env.cmd()
        .args(["repo", "bundle", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles"));
}

#[test]
fn add_bundle_to_project() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);
    env.create_bundle("proj-bundle", &["ref-a", "ref-b"]);
    env.init_project();

    env.cmd()
        .args(["add", "--bundle", "proj-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added bundle 'proj-bundle'"));

    let manifest = fs::read_to_string(env.project_dir.path().join("refstore.toml")).unwrap();
    assert!(
        manifest.contains("proj-bundle"),
        "manifest should contain bundle name"
    );
}

#[test]
fn add_unknown_bundle_fails() {
    let env = TestEnv::new();
    env.init_project();

    env.cmd()
        .args(["add", "--bundle", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn remove_bundle_from_project() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.create_bundle("rm-bundle", &["ref-a"]);
    env.init_project();

    env.cmd()
        .args(["add", "--bundle", "rm-bundle"])
        .assert()
        .success();

    env.cmd()
        .args(["remove", "--bundle", "rm-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed bundle"));

    let manifest = fs::read_to_string(env.project_dir.path().join("refstore.toml")).unwrap();
    assert!(
        !manifest.contains("rm-bundle"),
        "manifest should no longer contain bundle"
    );
}

#[test]
fn sync_resolves_bundles() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);
    env.create_bundle("sync-bundle", &["ref-a", "ref-b"]);
    env.init_project();

    env.cmd()
        .args(["add", "--bundle", "sync-bundle"])
        .assert()
        .success();

    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-a"))
        .stdout(predicate::str::contains("ref-b"))
        .stdout(predicate::str::contains("synced"));

    // Both references should be synced
    assert!(env.project_dir.path().join(".references/ref-a").exists());
    assert!(env.project_dir.path().join(".references/ref-b").exists());
    assert!(env.project_dir.path().join(".references/ref-a/README.md").exists());
}

#[test]
fn sync_explicit_overrides_bundle() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);
    env.create_bundle("override-bundle", &["ref-a", "ref-b"]);
    env.init_project();

    // Add bundle
    env.cmd()
        .args(["add", "--bundle", "override-bundle"])
        .assert()
        .success();

    // Also add ref-a explicitly with a custom path
    env.cmd()
        .args(["add", "ref-a", "--path", "custom-a"])
        .assert()
        .success();

    env.cmd().args(["sync"]).assert().success();

    // ref-a should be at the custom path (explicit overrides bundle default)
    assert!(env.project_dir.path().join(".references/custom-a").exists());
    assert!(env.project_dir.path().join(".references/custom-a/README.md").exists());
    // ref-a should NOT be at the default path
    assert!(!env.project_dir.path().join(".references/ref-a").exists());
    // ref-b from bundle should be at default path
    assert!(env.project_dir.path().join(".references/ref-b").exists());
}

#[test]
fn status_shows_bundle_info() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    env.add_repo_ref("ref-a", &sample);
    env.add_repo_ref("ref-b", &sample);
    env.create_bundle("status-bundle", &["ref-a", "ref-b"]);
    env.init_project();

    env.cmd()
        .args(["add", "--bundle", "status-bundle"])
        .assert()
        .success();

    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@status-bundle"))
        .stdout(predicate::str::contains("2 references"))
        .stdout(predicate::str::contains("via bundle"));
}

#[test]
fn full_bundle_workflow() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    // 1. Add references to central repo
    env.add_repo_ref("docs-a", &sample);
    env.add_repo_ref("docs-b", &sample);

    // 2. Create a bundle
    env.cmd()
        .args([
            "repo", "bundle", "create", "my-stack",
            "--ref", "docs-a", "--ref", "docs-b",
            "--description", "My full stack",
        ])
        .assert()
        .success();

    // 3. Verify bundle in list
    env.cmd()
        .args(["repo", "bundle", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-stack"));

    // 4. Init project
    env.init_project();

    // 5. Add bundle to project
    env.cmd()
        .args(["add", "--bundle", "my-stack"])
        .assert()
        .success();

    // 6. Status shows not synced
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not synced"));

    // 7. Sync
    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("synced"));

    // 8. Status shows synced
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("synced"))
        .stdout(predicate::str::contains("not synced").not());

    // 9. Content should exist
    assert!(env.project_dir.path().join(".references/docs-a/README.md").exists());
    assert!(env.project_dir.path().join(".references/docs-b/README.md").exists());

    // 10. Remove bundle with purge
    env.cmd()
        .args(["remove", "--bundle", "my-stack", "--purge"])
        .assert()
        .success();

    assert!(!env.project_dir.path().join(".references/docs-a").exists());
    assert!(!env.project_dir.path().join(".references/docs-b").exists());

    // 11. Clean up central repo
    env.cmd()
        .args(["repo", "bundle", "remove", "--force", "my-stack"])
        .assert()
        .success();

    env.cmd()
        .args(["repo", "bundle", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No bundles"));
}
