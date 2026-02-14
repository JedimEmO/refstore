use std::path::Path;
use std::process::Command;

use crate::error::RefstoreError;

pub fn ensure_git() -> Result<(), RefstoreError> {
    Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;
    Ok(())
}

pub fn clone_shallow(
    url: &str,
    target: &Path,
    git_ref: Option<&str>,
    depth: u32,
) -> Result<(), RefstoreError> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    if depth > 0 {
        cmd.args(["--depth", &depth.to_string()]);
    }
    cmd.arg("--single-branch");
    if let Some(r) = git_ref {
        cmd.args(["--branch", r]);
    }
    cmd.arg(url);
    cmd.arg(target);

    let output = cmd.output().map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }
    Ok(())
}

#[allow(dead_code)] // planned for incremental update support
pub fn pull(repo_path: &Path) -> Result<(), RefstoreError> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(repo_path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }
    Ok(())
}

pub fn head_hash(repo_path: &Path) -> Result<String, RefstoreError> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}
