use predicates::prelude::*;

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
        .args(["repo", "list"])
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
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-from-reg1"))
        .stdout(predicate::str::contains("ref-from-reg2"));
}
