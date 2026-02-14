use predicates::prelude::*;
use std::fs;

use crate::common::TestEnv;

#[test]
fn init_creates_manifest() {
    let env = TestEnv::new();

    env.cmd()
        .args(["init", "--no-self-ref", "--path"])
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
        .args(["init", "--no-self-ref", "--path"])
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
        .args(["init", "--no-self-ref", "--path"])
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
fn init_self_ref_creates_agents_md_when_none_exist() {
    let env = TestEnv::new();

    env.cmd()
        .args(["init", "--self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("added refstore instructions to AGENTS.md"));

    let agents_md = env.project_dir.path().join("AGENTS.md");
    assert!(agents_md.exists());
    let content = fs::read_to_string(&agents_md).unwrap();
    assert!(content.contains("refstore"));
    assert!(content.contains(".references/"));
    assert!(!env.project_dir.path().join("CLAUDE.md").exists());
}

#[test]
fn init_self_ref_appends_to_existing_claude_md() {
    let env = TestEnv::new();

    fs::write(
        env.project_dir.path().join("CLAUDE.md"),
        "# My Project\n\nExisting instructions.\n",
    )
    .unwrap();

    env.cmd()
        .args(["init", "--self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CLAUDE.md"));

    let content = fs::read_to_string(env.project_dir.path().join("CLAUDE.md")).unwrap();
    assert!(content.starts_with("# My Project"), "existing content preserved");
    assert!(content.contains("<!-- refstore -->"), "refstore section appended");
}

#[test]
fn init_self_ref_appends_to_existing_agents_md() {
    let env = TestEnv::new();

    fs::write(
        env.project_dir.path().join("AGENTS.md"),
        "# Agent Config\n",
    )
    .unwrap();

    env.cmd()
        .args(["init", "--self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("AGENTS.md"));

    let content = fs::read_to_string(env.project_dir.path().join("AGENTS.md")).unwrap();
    assert!(content.starts_with("# Agent Config"), "existing content preserved");
    assert!(content.contains("<!-- refstore -->"), "refstore section appended");
}

#[test]
fn init_self_ref_appends_to_both_when_both_exist() {
    let env = TestEnv::new();

    fs::write(env.project_dir.path().join("CLAUDE.md"), "# Claude\n").unwrap();
    fs::write(env.project_dir.path().join("AGENTS.md"), "# Agents\n").unwrap();

    env.cmd()
        .args(["init", "--self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CLAUDE.md"))
        .stdout(predicate::str::contains("AGENTS.md"));

    let claude = fs::read_to_string(env.project_dir.path().join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(env.project_dir.path().join("AGENTS.md")).unwrap();
    assert!(claude.contains("<!-- refstore -->"));
    assert!(agents.contains("<!-- refstore -->"));
}

#[test]
fn init_self_ref_idempotent() {
    let env = TestEnv::new();

    fs::write(env.project_dir.path().join("CLAUDE.md"), "# Project\n").unwrap();

    env.cmd()
        .args(["init", "--self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(env.project_dir.path().join("CLAUDE.md")).unwrap();
    assert_eq!(
        content.matches("<!-- refstore -->").count(),
        1,
        "marker should appear exactly once"
    );
}

#[test]
fn init_no_self_ref_skips() {
    let env = TestEnv::new();

    env.cmd()
        .args(["init", "--no-self-ref", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CLAUDE.md").not())
        .stdout(predicate::str::contains("AGENTS.md").not());

    assert!(!env.project_dir.path().join("CLAUDE.md").exists());
    assert!(!env.project_dir.path().join("AGENTS.md").exists());
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
fn install_mcp_creates_file() {
    let env = TestEnv::new();

    env.cmd()
        .args(["install-mcp", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'refstore'"));

    let mcp_path = env.project_dir.path().join(".mcp.json");
    assert!(mcp_path.exists());

    let content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&mcp_path).unwrap()).unwrap();
    let servers = content["mcpServers"].as_object().unwrap();
    assert!(servers.contains_key("refstore"));
    assert_eq!(servers["refstore"]["args"][0], "mcp");
}

#[test]
fn install_mcp_merges_existing() {
    let env = TestEnv::new();

    // Pre-create .mcp.json with another server
    let mcp_path = env.project_dir.path().join(".mcp.json");
    fs::write(
        &mcp_path,
        r#"{"mcpServers": {"other": {"command": "other-bin", "args": ["serve"]}}}"#,
    )
    .unwrap();

    env.cmd()
        .args(["install-mcp", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 'refstore'"));

    let content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&mcp_path).unwrap()).unwrap();
    let servers = content["mcpServers"].as_object().unwrap();
    assert!(servers.contains_key("refstore"), "refstore should be added");
    assert!(
        servers.contains_key("other"),
        "existing server should be preserved"
    );
}

#[test]
fn install_mcp_already_configured() {
    let env = TestEnv::new();

    // Install once
    env.cmd()
        .args(["install-mcp", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success();

    // Install again — should not duplicate
    env.cmd()
        .args(["install-mcp", "--path"])
        .arg(env.project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already configured"));
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
