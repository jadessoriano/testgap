use crate::{Result, TestGapError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the set of changed files as paths relative to `analysis_root`.
///
/// Resolves the git repo root internally so this works correctly even when
/// `analysis_root` is a subdirectory of the repository (monorepo case).
///
/// Combines:
/// - `git diff --name-only <base_ref>` (committed changes vs base)
/// - `git diff --name-only` (unstaged changes)
/// - `git ls-files --others --exclude-standard` (untracked files)
pub fn get_changed_files(analysis_root: &Path, base_ref: &str) -> Result<HashSet<PathBuf>> {
    let git_root = find_git_root(analysis_root)?;

    let mut changed_repo_relative = HashSet::new();

    // All git commands run from the repo root so paths are consistently repo-relative.

    // Committed changes relative to base ref
    let output = Command::new("git")
        .args(["diff", "--name-only", base_ref])
        .current_dir(&git_root)
        .output()
        .map_err(|e| TestGapError::Config(format!("failed to run git diff: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TestGapError::Config(format!(
            "git diff --name-only {base_ref} failed: {stderr}"
        )));
    }

    collect_lines(&output.stdout, &mut changed_repo_relative);

    // Unstaged changes in working tree
    let output = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(&git_root)
        .output()
        .map_err(|e| TestGapError::Config(format!("failed to run git diff: {e}")))?;

    if output.status.success() {
        collect_lines(&output.stdout, &mut changed_repo_relative);
    }

    // Untracked files (also from repo root for consistency)
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(&git_root)
        .output()
        .map_err(|e| TestGapError::Config(format!("failed to run git ls-files: {e}")))?;

    if output.status.success() {
        collect_lines(&output.stdout, &mut changed_repo_relative);
    }

    // Convert repo-relative paths to analysis-root-relative paths.
    // If analysis_root == git_root, prefix is empty and paths pass through unchanged.
    let prefix = analysis_root
        .strip_prefix(&git_root)
        .unwrap_or(Path::new(""));

    let mut result = HashSet::new();
    for repo_path in changed_repo_relative {
        if let Ok(rel) = repo_path.strip_prefix(prefix) {
            result.insert(rel.to_path_buf());
        }
        // Paths outside the analysis root are silently ignored
    }

    Ok(result)
}

/// Resolve the default branch name for a repository.
///
/// Tries `origin/HEAD` first (works after `git clone`), then falls back to
/// checking if `main` or `master` exist as local branches.
pub fn resolve_default_branch(start: &Path) -> Result<String> {
    // Try origin/HEAD (set by git clone)
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(start)
        .output();

    if let Ok(ref out) = output {
        if out.status.success() {
            let full_ref = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // "refs/remotes/origin/main" → "origin/main"
            if let Some(branch) = full_ref.strip_prefix("refs/remotes/") {
                return Ok(branch.to_string());
            }
        }
    }

    // Fallback: check if main or master branches exist
    for candidate in &["main", "master"] {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", candidate])
            .current_dir(start)
            .output();
        if let Ok(ref out) = output {
            if out.status.success() {
                return Ok((*candidate).to_string());
            }
        }
    }

    Err(TestGapError::Config(
        "could not detect default branch (tried origin/HEAD, main, master)".into(),
    ))
}

/// Find the git repository root from a starting directory.
fn find_git_root(start: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
        .map_err(|e| {
            TestGapError::Config(format!("not a git repository (failed to run git): {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TestGapError::Config(format!(
            "not a git repository: {stderr}"
        )));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn collect_lines(stdout: &[u8], set: &mut HashSet<PathBuf>) {
    for line in String::from_utf8_lossy(stdout).lines() {
        let line = line.trim();
        if !line.is_empty() {
            set.insert(PathBuf::from(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn invalid_ref_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let result = get_changed_files(dir.path(), "nonexistent-ref-abc123");
        assert!(result.is_err(), "expected error for invalid ref");
    }

    #[test]
    fn detects_modified_file() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        std::fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Modify the file
        std::fs::write(dir.path().join("hello.rs"), "fn main() { println!(); }").unwrap();

        let changed = get_changed_files(dir.path(), "HEAD").unwrap();
        assert!(
            changed.contains(&PathBuf::from("hello.rs")),
            "expected hello.rs in changed set, got: {changed:?}"
        );
    }

    #[test]
    fn subdirectory_returns_relative_to_analysis_root() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        // Create a subdirectory structure like a monorepo
        let sub = dir.path().join("crates").join("mylib").join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("lib.rs"), "pub fn foo() {}").unwrap();
        std::fs::write(dir.path().join("root.rs"), "fn root() {}").unwrap();

        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Modify the file in the subdirectory
        std::fs::write(sub.join("lib.rs"), "pub fn foo() { 42 }").unwrap();

        // Analyze from the subdirectory, not repo root
        let analysis_root = dir.path().join("crates").join("mylib");
        let changed = get_changed_files(&analysis_root, "HEAD").unwrap();

        // Should contain the path relative to the analysis root, not repo root
        assert!(
            changed.contains(&PathBuf::from("src/lib.rs")),
            "expected src/lib.rs relative to analysis root, got: {changed:?}"
        );
        // root.rs is outside analysis root, should NOT appear
        assert!(
            !changed.contains(&PathBuf::from("root.rs")),
            "root.rs should be excluded (outside analysis root)"
        );
    }

    #[test]
    fn non_git_directory_returns_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = get_changed_files(dir.path(), "main");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not a git repository"),
            "expected clear error message, got: {err}"
        );
    }
}
