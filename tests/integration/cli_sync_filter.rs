use predicates::prelude::*;

use crate::common::TestEnv;

/// Helper: set up a repo ref, init project, add ref with include/exclude globs, then sync.
fn setup_and_sync(env: &TestEnv, includes: &[&str], excludes: &[&str]) {
    let sample = env.create_sample_files();
    env.add_repo_ref("filtered", &sample);
    env.init_project();

    let mut cmd = env.cmd();
    cmd.args(["add", "filtered"]);
    for inc in includes {
        cmd.args(["--include", inc]);
    }
    for exc in excludes {
        cmd.args(["--exclude", exc]);
    }
    cmd.assert().success();

    env.cmd().args(["sync"]).assert().success();
}

#[test]
fn sync_include_filter() {
    let env = TestEnv::new();
    setup_and_sync(&env, &["*.md"], &[]);

    let ref_dir = env.project_dir.path().join(".references/filtered");

    // Markdown files should be synced
    assert!(ref_dir.join("README.md").exists());
    assert!(ref_dir.join("docs/guide.md").exists());

    // Non-markdown files should NOT be synced
    assert!(!ref_dir.join("src/lib.rs").exists());
    assert!(!ref_dir.join("src/util.rs").exists());
    assert!(!ref_dir.join("docs/notes.txt").exists());
}

#[test]
fn sync_exclude_filter() {
    let env = TestEnv::new();
    setup_and_sync(&env, &[], &["*.txt"]);

    let ref_dir = env.project_dir.path().join(".references/filtered");

    // txt files should be excluded
    assert!(!ref_dir.join("docs/notes.txt").exists());

    // Everything else should be synced
    assert!(ref_dir.join("README.md").exists());
    assert!(ref_dir.join("src/lib.rs").exists());
    assert!(ref_dir.join("docs/guide.md").exists());
}

#[test]
fn sync_include_and_exclude() {
    let env = TestEnv::new();
    // Include only src/*, but exclude util.rs
    setup_and_sync(&env, &["src/*"], &["*/util.rs"]);

    let ref_dir = env.project_dir.path().join(".references/filtered");

    assert!(ref_dir.join("src/lib.rs").exists());
    assert!(!ref_dir.join("src/util.rs").exists());
    assert!(!ref_dir.join("README.md").exists());
    assert!(!ref_dir.join("docs/guide.md").exists());
}

#[test]
fn sync_nested_include() {
    let env = TestEnv::new();
    setup_and_sync(&env, &["**/*.md"], &[]);

    let ref_dir = env.project_dir.path().join(".references/filtered");

    // Both top-level and nested .md files should be synced
    assert!(ref_dir.join("README.md").exists());
    assert!(ref_dir.join("docs/guide.md").exists());

    // Non-md files should not
    assert!(!ref_dir.join("src/lib.rs").exists());
    assert!(!ref_dir.join("docs/notes.txt").exists());
}

#[test]
fn sync_no_filters_copies_all() {
    let env = TestEnv::new();
    setup_and_sync(&env, &[], &[]);

    let ref_dir = env.project_dir.path().join(".references/filtered");

    env.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("filtered"));

    assert!(ref_dir.join("README.md").exists());
    assert!(ref_dir.join("src/lib.rs").exists());
    assert!(ref_dir.join("src/util.rs").exists());
    assert!(ref_dir.join("docs/guide.md").exists());
    assert!(ref_dir.join("docs/notes.txt").exists());
}
