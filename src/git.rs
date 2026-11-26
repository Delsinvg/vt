use std::path::{Path, PathBuf};
use std::process::Command;

const GIT_DIRECTORY: &str = ".git";

pub fn repository_root(directory: &Path) -> Option<PathBuf> {
    let common_directory = PathBuf::from(git(
        directory,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    if common_directory.ends_with(GIT_DIRECTORY) {
        common_directory.parent().map(Path::to_path_buf)
    } else {
        Some(common_directory)
    }
}

pub fn worktree_root(directory: &Path) -> Option<PathBuf> {
    git(directory, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

pub fn current_branch(directory: &Path) -> Option<String> {
    git(directory, &["branch", "--show-current"]).filter(|branch| !branch.is_empty())
}

fn git(directory: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn run_git(directory: &Path, arguments: &[&str]) {
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
        run_git(directory.path(), &["init", "--quiet"]);
        run_git(
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
        run_git(
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
    fn knows_the_branch_and_the_worktree_folder() {
        let repo = repository();
        let worktrees = TempDir::new().unwrap();
        let worktree = worktrees.path().join("feature");
        run_git(
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

        assert_eq!(current_branch(&worktree), Some("feature".to_string()));
        assert_eq!(
            worktree_root(&worktree).map(|root| fs::canonicalize(root).unwrap()),
            Some(fs::canonicalize(&worktree).unwrap())
        );
    }

    #[test]
    fn a_folder_outside_git_has_no_repository() {
        let folder = TempDir::new().unwrap();

        assert_eq!(repository_root(folder.path()), None);
    }

    #[test]
    fn a_worktree_of_a_bare_repository_belongs_to_the_bare_repository() {
        let origin = repository();
        let parent = TempDir::new().unwrap();
        let bare = parent.path().join("garden.git");
        let worktree = parent.path().join("feature");
        run_git(
            parent.path(),
            &[
                "clone",
                "--bare",
                "--quiet",
                origin.path().to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_git(
            &bare,
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
            Some(fs::canonicalize(&bare).unwrap())
        );
    }
}
