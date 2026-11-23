use std::path::{Path, PathBuf};
use std::process::Command;

const GIT_DIRECTORY: &str = ".git";

pub fn repository_root(directory: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let common_directory = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    if common_directory.ends_with(GIT_DIRECTORY) {
        common_directory.parent().map(Path::to_path_buf)
    } else {
        Some(common_directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn git(directory: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(arguments)
            .output()
            .unwrap()
            .status;
        assert!(status.success(), "git {arguments:?} failed");
    }

    fn repository() -> TempDir {
        let directory = TempDir::new().unwrap();
        git(directory.path(), &["init", "--quiet"]);
        git(
            directory.path(),
            &["commit", "--quiet", "--allow-empty", "-m", "start"],
        );
        directory
    }

    #[test]
    fn finds_the_root_from_a_subfolder() {
        let repo = repository();
        let subfolder = repo.path().join("src/deep");
        fs::create_dir_all(&subfolder).unwrap();

        assert_eq!(
            repository_root(&subfolder).map(|root| fs::canonicalize(root).unwrap()),
            Some(fs::canonicalize(repo.path()).unwrap())
        );
    }

    #[test]
    fn a_worktree_belongs_to_its_main_repository() {
        let repo = repository();
        let worktrees = TempDir::new().unwrap();
        let worktree = worktrees.path().join("feature");
        git(
            repo.path(),
            &[
                "worktree",
                "add",
                "--quiet",
                worktree.to_str().unwrap(),
                "-b",
                "feature",
            ],
        );

        assert_eq!(
            repository_root(&worktree).map(|root| fs::canonicalize(root).unwrap()),
            Some(fs::canonicalize(repo.path()).unwrap())
        );
    }

    #[test]
    fn a_folder_outside_git_has_no_repository() {
        let folder = TempDir::new().unwrap();

        assert_eq!(repository_root(folder.path()), None);
    }
}
