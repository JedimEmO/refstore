use predicates::prelude::*;
use std::fs;

use crate::common::TestEnv;

#[test]
fn registry_add_and_list() {
    let env = TestEnv::new();
    let reg_dir = env.create_fake_registry(&[("remote-ref", "# Remote\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added registry 'test-reg'"));

    env.cmd()
        .args(["registry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local:"))
        .stdout(predicate::str::contains("test-reg: 1 references"));
}

#[test]
fn registry_references_appear_in_repo_list() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    let reg_dir = env.create_fake_registry(&[("remote-ref", "# Remote\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    // Add a local reference
    env.add_repo_ref("local-ref", &sample);

    // Add a remote registry
    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success();

    // Both local and remote should appear in repo list
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local-ref"))
        .stdout(predicate::str::contains("remote-ref"));
}

#[test]
fn local_reference_wins_over_remote() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();

    // Create a remote registry with a reference named "shared"
    let reg_dir = env.create_fake_registry(&[("shared", "# Remote version\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    // Add a local reference with the same name
    env.add_repo_ref("shared", &sample);

    // Add the remote registry
    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success();

    // Sync to project: should use local version (has src/lib.rs), not remote (has README.md only)
    env.init_project();
    env.cmd()
        .args(["add", "shared"])
        .assert()
        .success();
    env.cmd()
        .args(["sync"])
        .assert()
        .success();

    // The synced content should be from local (which has src/lib.rs)
    let synced = env.project_dir.path().join(".references/shared/src/lib.rs");
    assert!(synced.exists(), "local reference should win over remote");
}

#[test]
fn sync_remote_registry_reference() {
    let env = TestEnv::new();
    let reg_dir = env.create_fake_registry(&[("remote-docs", "# Remote Docs\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success();

    env.init_project();
    env.cmd()
        .args(["add", "remote-docs"])
        .assert()
        .success();
    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote-docs: synced"));

    // Content should be from the remote registry
    let synced = env.project_dir.path().join(".references/remote-docs/README.md");
    assert!(synced.exists(), "remote reference content should be synced");
    let content = std::fs::read_to_string(synced).unwrap();
    assert_eq!(content, "# Remote Docs\n");
}

#[test]
fn registry_remove() {
    let env = TestEnv::new();
    let reg_dir = env.create_fake_registry(&[("remote-ref", "# Remote\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success();

    env.cmd()
        .args(["registry", "remove", "--force", "test-reg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed registry 'test-reg'"));

    // Should no longer appear in list
    env.cmd()
        .args(["registry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-reg").not());
}

#[test]
fn registry_remove_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["registry", "remove", "--force", "nonexistent"])
        .assert()
        .failure();
}

#[test]
fn registry_add_duplicate_fails() {
    let env = TestEnv::new();
    let reg_dir = env.create_fake_registry(&[("ref1", "# Ref\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .success();

    env.cmd()
        .args(["registry", "add", "test-reg", &reg_url])
        .assert()
        .failure();
}

#[test]
fn registry_local_name_reserved() {
    let env = TestEnv::new();

    env.cmd()
        .args(["registry", "add", "local", "https://example.com/reg.git"])
        .assert()
        .failure();
}

#[test]
fn multiple_registries() {
    let env = TestEnv::new();
    let reg1 = env.create_fake_registry(&[("ref-from-reg1", "# Reg1\n")]);
    // Create a second registry in a different location
    let reg2_dir = env.project_dir.path().join("fake-registry-2");
    std::fs::create_dir_all(reg2_dir.join("content/ref-from-reg2")).unwrap();
    std::fs::write(reg2_dir.join("content/ref-from-reg2/README.md"), "# Reg2\n").unwrap();
    std::fs::write(
        reg2_dir.join("index.toml"),
        r#"version = 1
[references.ref-from-reg2]
name = "ref-from-reg2"
kind = "directory"
added_at = "2026-01-01T00:00:00Z"
[references.ref-from-reg2.source]
type = "local"
path = "/fake"
"#,
    )
    .unwrap();
    for args in [
        &["init"][..],
        &["config", "user.name", "test"],
        &["config", "user.email", "test@test"],
        &["add", "."],
        &["commit", "-m", "init"],
    ] {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&reg2_dir)
            .output()
            .unwrap();
    }

    let reg1_url = format!("file://{}", reg1.display());
    let reg2_url = format!("file://{}", reg2_dir.display());

    env.cmd()
        .args(["registry", "add", "reg1", &reg1_url])
        .assert()
        .success();
    env.cmd()
        .args(["registry", "add", "reg2", &reg2_url])
        .assert()
        .success();

    // Both refs should appear
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-from-reg1"))
        .stdout(predicate::str::contains("ref-from-reg2"));
}

// --- registry init ---

#[test]
fn registry_init_creates_valid_registry() {
    let env = TestEnv::new();
    let reg_path = env.project_dir.path().join("my-registry");

    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized registry"));

    // Should have the expected structure
    assert!(reg_path.join("index.toml").exists(), "index.toml should exist");
    assert!(reg_path.join("content").is_dir(), "content/ dir should exist");
    assert!(reg_path.join(".git").is_dir(), "should be a git repo");
}

#[test]
fn registry_init_is_usable_as_remote() {
    let env = TestEnv::new();
    let reg_path = env.project_dir.path().join("shared-registry");

    // Init a new registry
    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    // Add it as a remote registry via file:// URL
    let reg_url = format!("file://{}", reg_path.display());
    env.cmd()
        .args(["registry", "add", "shared", &reg_url])
        .assert()
        .success();

    // Should appear in list with 0 references
    env.cmd()
        .args(["registry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shared: 0 references"));
}

#[test]
fn registry_init_can_receive_content_via_data_dir() {
    let env = TestEnv::new();
    let reg_path = env.project_dir.path().join("authored-registry");
    let sample = env.create_sample_files();

    // Init a new registry
    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    // Add a reference directly to it using --data-dir
    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(&reg_path)
        .current_dir(env.project_dir.path())
        .args(["store", "add", "my-docs"])
        .arg(&sample)
        .assert()
        .success();

    // Content should be in the registry
    assert!(
        reg_path.join("content/my-docs/README.md").exists(),
        "reference content should be in the registry"
    );

    // The registry should now list 1 reference
    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(&reg_path)
        .current_dir(env.project_dir.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-docs"));
}

// --- store push ---

#[test]
fn store_push_copies_reference_to_target_registry() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    let reg_path = env.project_dir.path().join("target-registry");

    // Add a reference to the local store
    env.add_repo_ref("push-me", &sample);

    // Init a target registry
    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    // Push the reference to the target registry
    env.cmd()
        .args(["store", "push", "push-me", "--to"])
        .arg(&reg_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Pushed 'push-me'"));

    // Verify content exists in the target registry
    assert!(
        reg_path.join("content/push-me/README.md").exists(),
        "pushed content should exist in target"
    );
    assert!(
        reg_path.join("content/push-me/src/lib.rs").exists(),
        "pushed content should include nested files"
    );

    // Verify index.toml in target was updated
    let index = fs::read_to_string(reg_path.join("index.toml")).unwrap();
    assert!(index.contains("push-me"), "target index should contain the reference");
}

#[test]
fn store_push_duplicate_fails() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    let reg_path = env.project_dir.path().join("target-registry");

    env.add_repo_ref("dup-push", &sample);

    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    // Push once — should succeed
    env.cmd()
        .args(["store", "push", "dup-push", "--to"])
        .arg(&reg_path)
        .assert()
        .success();

    // Push again — should fail (already exists)
    env.cmd()
        .args(["store", "push", "dup-push", "--to"])
        .arg(&reg_path)
        .assert()
        .failure();
}

#[test]
fn store_push_nonexistent_fails() {
    let env = TestEnv::new();
    let reg_path = env.project_dir.path().join("target-registry");

    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    env.cmd()
        .args(["store", "push", "ghost-ref", "--to"])
        .arg(&reg_path)
        .assert()
        .failure();
}

// --- registry update ---

#[test]
fn registry_update_pulls_new_content() {
    let env = TestEnv::new();

    // Create a remote registry with one reference
    let reg_dir = env.create_fake_registry(&[("original-ref", "# Original\n")]);
    let reg_url = format!("file://{}", reg_dir.display());

    // Add it as a remote
    env.cmd()
        .args(["registry", "add", "updatable-reg", &reg_url])
        .assert()
        .success();

    // Verify initial state
    env.cmd()
        .args(["registry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updatable-reg: 1 references"));

    // Add a second reference to the source registry
    let new_ref_dir = reg_dir.join("content/new-ref");
    fs::create_dir_all(&new_ref_dir).unwrap();
    fs::write(new_ref_dir.join("README.md"), "# New\n").unwrap();

    let index = fs::read_to_string(reg_dir.join("index.toml")).unwrap();
    let updated_index = format!(
        "{index}\n[references.new-ref]\nname = \"new-ref\"\nkind = \"directory\"\nadded_at = \"2026-01-01T00:00:00Z\"\n\n[references.new-ref.source]\ntype = \"local\"\npath = \"/fake\"\n"
    );
    fs::write(reg_dir.join("index.toml"), updated_index).unwrap();

    // Commit the changes in the source registry
    for args in [
        &["add", "."][..],
        &["commit", "-m", "add new-ref"],
    ] {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&reg_dir)
            .output()
            .unwrap();
    }

    // Update the remote registry
    env.cmd()
        .args(["registry", "update", "updatable-reg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated registry"));

    // Should now show 2 references
    env.cmd()
        .args(["registry", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updatable-reg: 2 references"));
}

// --- full round-trip workflow ---

#[test]
fn full_registry_authoring_workflow() {
    let env = TestEnv::new();
    let sample = env.create_sample_files();
    let reg_path = env.project_dir.path().join("team-registry");

    // 1. Add references to local store
    env.add_repo_ref("team-guidelines", &sample);

    // 2. Create a new registry
    env.cmd()
        .args(["registry", "init"])
        .arg(&reg_path)
        .assert()
        .success();

    // 3. Push references to the registry
    env.cmd()
        .args(["store", "push", "team-guidelines", "--to"])
        .arg(&reg_path)
        .assert()
        .success();

    // 4. A different "user" (fresh data dir) adds the registry and syncs
    let consumer_data = tempfile::TempDir::new().unwrap();
    let consumer_project = tempfile::TempDir::new().unwrap();
    let reg_url = format!("file://{}", reg_path.display());

    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(consumer_data.path())
        .current_dir(consumer_project.path())
        .args(["registry", "add", "team", &reg_url])
        .assert()
        .success();

    // 5. Consumer can see the reference
    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(consumer_data.path())
        .current_dir(consumer_project.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("team-guidelines"));

    // 6. Consumer inits project, adds the reference, and syncs
    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(consumer_data.path())
        .current_dir(consumer_project.path())
        .args(["init", "--no-self-ref", "--path"])
        .arg(consumer_project.path())
        .assert()
        .success();

    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(consumer_data.path())
        .current_dir(consumer_project.path())
        .args(["add", "team-guidelines"])
        .assert()
        .success();

    let mut cmd = assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
    cmd.arg("--data-dir")
        .arg(consumer_data.path())
        .current_dir(consumer_project.path())
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("team-guidelines: synced"));

    // 7. Content should be synced from the authored registry
    let synced = consumer_project.path().join(".references/team-guidelines/README.md");
    assert!(synced.exists(), "consumer should have synced content");
    let content = fs::read_to_string(synced).unwrap();
    assert_eq!(content, "# Sample Reference\n");
}
