use std::fs;
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

/// Initialize a git repo at `path` if one doesn't already exist.
/// Sets up minimal config (user.name/email) so commits work in any environment.
pub fn init(path: &Path) -> Result<(), RefstoreError> {
    if is_git_repo(path) {
        return Ok(());
    }

    ensure_git()?;

    let output = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }

    // Set local config so commits work even without global git config
    run_git(path, &["config", "user.name", "refstore"])?;
    run_git(path, &["config", "user.email", "refstore@local"])?;

    Ok(())
}

/// Stage specific paths and create a commit.
/// `paths` are relative to `repo_path`.
pub fn commit(repo_path: &Path, paths: &[&str], message: &str) -> Result<(), RefstoreError> {
    // Stage the specified paths
    for path in paths {
        let mut cmd = Command::new("git");
        cmd.args(["add", path]).current_dir(repo_path);
        let output = cmd.output().map_err(|_| RefstoreError::GitNotFound)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RefstoreError::GitCommand(stderr.to_string()));
        }
    }

    // Check if there's anything to commit (avoid empty commits)
    let status_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(repo_path)
        .status()
        .map_err(|_| RefstoreError::GitNotFound)?;

    // Exit code 0 means no staged changes - nothing to commit
    if status_output.success() {
        return Ok(());
    }

    // Commit
    run_git(repo_path, &["commit", "-m", message])?;

    Ok(())
}

/// Stage removals (deleted files) and create a commit.
pub fn commit_removals(repo_path: &Path, paths: &[&str], message: &str) -> Result<(), RefstoreError> {
    for path in paths {
        // Use `git add -A` on the path to pick up deletions
        let mut cmd = Command::new("git");
        cmd.args(["add", "-A", path]).current_dir(repo_path);
        let output = cmd.output().map_err(|_| RefstoreError::GitNotFound)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RefstoreError::GitCommand(stderr.to_string()));
        }
    }

    let status_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(repo_path)
        .status()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if status_output.success() {
        return Ok(());
    }

    run_git(repo_path, &["commit", "-m", message])?;

    Ok(())
}

/// Ensure `.gitignore` at `repo_path` contains all given patterns.
/// Creates the file if it doesn't exist. Appends missing patterns.
pub fn ensure_gitignore(repo_path: &Path, patterns: &[&str]) -> Result<(), RefstoreError> {
    let gitignore_path = repo_path.join(".gitignore");
    let existing = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).map_err(|source| RefstoreError::FileRead {
            path: gitignore_path.clone(),
            source,
        })?
    } else {
        String::new()
    };

    let existing_lines: Vec<&str> = existing.lines().collect();
    let mut to_add = Vec::new();

    for pattern in patterns {
        if !existing_lines.iter().any(|line| line.trim() == *pattern) {
            to_add.push(*pattern);
        }
    }

    if !to_add.is_empty() {
        let mut content = existing;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        for pattern in to_add {
            content.push_str(pattern);
            content.push('\n');
        }
        fs::write(&gitignore_path, content).map_err(|source| RefstoreError::FileWrite {
            path: gitignore_path,
            source,
        })?;
    }

    Ok(())
}

/// Add a git submodule.
pub fn submodule_add(repo_path: &Path, url: &str, path: &str) -> Result<(), RefstoreError> {
    // -c protocol.file.allow=always is needed for file:// URLs (local registries, testing)
    run_git(repo_path, &["-c", "protocol.file.allow=always", "submodule", "add", url, path])
}

/// Remove a git submodule.
pub fn submodule_remove(repo_path: &Path, path: &str) -> Result<(), RefstoreError> {
    run_git(repo_path, &["submodule", "deinit", "-f", path])?;
    run_git(repo_path, &["rm", "-f", path])?;
    // Clean up .git/modules/<path> if it exists
    let modules_dir = repo_path.join(".git").join("modules").join(path);
    if modules_dir.exists() {
        let _ = fs::remove_dir_all(&modules_dir);
    }
    Ok(())
}

/// Update submodule(s) to latest remote commit.
/// If `path` is Some, only update that submodule; otherwise update all.
pub fn submodule_update(repo_path: &Path, path: Option<&str>) -> Result<(), RefstoreError> {
    let mut args = vec!["-c", "protocol.file.allow=always", "submodule", "update", "--remote"];
    if let Some(p) = path {
        args.push(p);
    }
    run_git(repo_path, &args)
}

/// Remove `.git/` directory from a path, turning a git clone into plain files.
pub fn strip_git_dir(path: &Path) -> Result<(), RefstoreError> {
    let git_dir = path.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir).map_err(|source| RefstoreError::DirCreate {
            path: git_dir,
            source,
        })?;
    }
    Ok(())
}

/// A single entry from `git log` for a specific path.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub hash: String,
    pub date: String,
    pub message: String,
}

/// Get git log entries for a specific path within a repo.
/// Returns entries from newest to oldest.
pub fn log_path(repo_path: &Path, path: &str) -> Result<Vec<LogEntry>, RefstoreError> {
    let output = Command::new("git")
        .args(["log", "--format=%H|%aI|%s", "--", path])
        .current_dir(repo_path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }

    let entries = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                Some(LogEntry {
                    hash: parts[0].to_string(),
                    date: parts[1].to_string(),
                    message: parts[2].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}

/// Extract content at a specific git ref into a destination directory.
/// `content_path` is the path within the repo (e.g., "content/my-ref").
/// Files are extracted to `dest` with the `content_path` prefix stripped.
pub fn archive_path_at_ref(
    repo_path: &Path,
    git_ref: &str,
    content_path: &str,
    dest: &Path,
) -> Result<(), RefstoreError> {
    fs::create_dir_all(dest).map_err(|source| RefstoreError::DirCreate {
        path: dest.to_path_buf(),
        source,
    })?;

    // Count path components to strip (e.g., "content/my-ref" = 2)
    let strip_components = content_path.split('/').count();

    let git_archive = Command::new("git")
        .args(["archive", git_ref, "--", content_path])
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| RefstoreError::GitNotFound)?;

    let output = Command::new("tar")
        .args(["x", &format!("--strip-components={strip_components}")])
        .current_dir(dest)
        .stdin(git_archive.stdout.unwrap())
        .output()
        .map_err(|e| RefstoreError::GitCommand(format!("tar extraction failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(format!(
            "failed to extract content at ref '{git_ref}': {stderr}"
        )));
    }

    Ok(())
}

/// Check if a git ref (tag, branch, commit hash) exists in the repo.
pub fn ref_exists(repo_path: &Path, git_ref: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", &format!("{git_ref}^{{commit}}")])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List tags in a repo.
pub fn list_tags(repo_path: &Path) -> Result<Vec<String>, RefstoreError> {
    let output = Command::new("git")
        .args(["tag", "--sort=-creatordate"])
        .current_dir(repo_path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Create a tag in a repo.
pub fn create_tag(repo_path: &Path, tag: &str, message: Option<&str>) -> Result<(), RefstoreError> {
    match message {
        Some(msg) => run_git(repo_path, &["tag", "-a", tag, "-m", msg]),
        None => run_git(repo_path, &["tag", tag]),
    }
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

/// Run a git command and return an error if it fails.
fn run_git(repo_path: &Path, args: &[&str]) -> Result<(), RefstoreError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|_| RefstoreError::GitNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RefstoreError::GitCommand(stderr.to_string()));
    }
    Ok(())
}
