use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

pub struct TestEnv {
    pub data_dir: TempDir,
    pub project_dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            data_dir: TempDir::new().expect("failed to create data_dir"),
            project_dir: TempDir::new().expect("failed to create project_dir"),
        }
    }

    /// Build a refstore Command pre-configured with --data-dir and cwd = project_dir.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("refstore"));
        cmd.arg("--data-dir")
            .arg(self.data_dir.path())
            .current_dir(self.project_dir.path());
        cmd
    }

    /// Create sample files inside a subdirectory of project_dir and return that path.
    /// Structure:
    ///   sample/
    ///     README.md
    ///     src/
    ///       lib.rs
    ///       util.rs
    ///     docs/
    ///       guide.md
    ///       notes.txt
    pub fn create_sample_files(&self) -> PathBuf {
        let root = self.project_dir.path().join("sample");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("README.md"), "# Sample Reference\n").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        fs::write(root.join("src/util.rs"), "pub fn util() {}\n").unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();
        fs::write(root.join("docs/notes.txt"), "Some notes\n").unwrap();
        root
    }

    /// Create a single sample file and return its path.
    pub fn create_sample_file(&self) -> PathBuf {
        let file = self.project_dir.path().join("sample.txt");
        fs::write(&file, "hello world\n").unwrap();
        file
    }

    /// Shorthand: add a local directory reference to the central repo.
    pub fn add_repo_ref(&self, name: &str, source: &Path) {
        self.cmd()
            .args(["repo", "add", name])
            .arg(source)
            .assert()
            .success();
    }

    /// Shorthand: add a local directory ref with tags and description.
    pub fn add_repo_ref_with_meta(
        &self,
        name: &str,
        source: &Path,
        description: &str,
        tags: &[&str],
    ) {
        let mut cmd = self.cmd();
        cmd.args(["repo", "add", name]);
        cmd.arg(source);
        cmd.args(["--description", description]);
        for tag in tags {
            cmd.args(["--tag", tag]);
        }
        cmd.assert().success();
    }

    /// Shorthand: create a bundle in the central repo from existing references.
    pub fn create_bundle(&self, name: &str, refs: &[&str]) {
        let mut cmd = self.cmd();
        cmd.args(["repo", "bundle", "create", name]);
        for r in refs {
            cmd.args(["--ref", r]);
        }
        cmd.assert().success();
    }

    /// Shorthand: init project in project_dir (skips self-ref prompt).
    pub fn init_project(&self) {
        self.cmd()
            .args(["init", "--no-self-ref", "--path"])
            .arg(self.project_dir.path())
            .assert()
            .success();
    }
}
