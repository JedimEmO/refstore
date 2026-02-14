use predicates::prelude::*;

use crate::common::TestEnv;

#[test]
fn config_show_defaults() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MCP scope"))
        .stdout(predicate::str::contains("read_only"))
        .stdout(predicate::str::contains("Git depth"))
        .stdout(predicate::str::contains("1"));
}

#[test]
fn config_set_mcp_scope() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "set", "mcp_scope", "read_write"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set mcp_scope = read_write"));

    env.cmd()
        .args(["config", "get", "mcp_scope"])
        .assert()
        .success()
        .stdout(predicate::str::contains("read_write"));
}

#[test]
fn config_set_git_depth() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "set", "git_depth", "5"])
        .assert()
        .success();

    env.cmd()
        .args(["config", "get", "git_depth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("5"));
}

#[test]
fn config_set_default_branch() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "set", "default_branch", "develop"])
        .assert()
        .success();

    env.cmd()
        .args(["config", "get", "default_branch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("develop"));
}

#[test]
fn config_set_invalid_key() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "set", "nonexistent_key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown config key"));
}

#[test]
fn config_set_invalid_value() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "set", "mcp_scope", "garbage"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid mcp_scope"));
}
