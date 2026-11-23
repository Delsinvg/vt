use std::fs;
use std::path::{Path, PathBuf};

const FRONTMATTER_FENCE: &str = "---";
const REPOS_KEY: &str = "repos:";
const HOME_PREFIX: &str = "~/";

#[derive(Debug, PartialEq)]
pub struct Project {
    pub note: PathBuf,
    pub repos: Vec<PathBuf>,
}

impl Project {
    pub fn folder(&self) -> PathBuf {
        self.note
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }
}

pub fn repos_in(content: &str, home: &Path) -> Vec<PathBuf> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some(FRONTMATTER_FENCE) {
        return Vec::new();
    }

    let mut repos = Vec::new();
    let mut inside_repos = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == FRONTMATTER_FENCE {
            break;
        }
        if trimmed.starts_with(REPOS_KEY) {
            inside_repos = true;
        } else if let Some(repo) = trimmed.strip_prefix("- ").filter(|_| inside_repos) {
            repos.push(expand_home(repo.trim(), home));
        } else if !trimmed.is_empty() {
            inside_repos = false;
        }
    }
    repos
}

pub fn folder_for(projects: &[Project], repository: &Path) -> Option<PathBuf> {
    let repository = canonical(repository);
    projects
        .iter()
        .find(|project| {
            project
                .repos
                .iter()
                .any(|repo| canonical(repo) == repository)
        })
        .map(Project::folder)
}

fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix(HOME_PREFIX) {
        Some(inside_home) => home.join(inside_home),
        None => PathBuf::from(path),
    }
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/someone";

    fn repos(content: &str) -> Vec<PathBuf> {
        repos_in(content, Path::new(HOME))
    }

    #[test]
    fn reads_the_repos_from_the_frontmatter() {
        let content = "---\nrepos:\n  - ~/code/garden\n  - /srv/garden-infra\n---\n# Garden\n";

        assert_eq!(
            repos(content),
            vec![
                PathBuf::from("/home/someone/code/garden"),
                PathBuf::from("/srv/garden-infra"),
            ]
        );
    }

    #[test]
    fn stops_at_the_next_key() {
        let content = "---\nrepos:\n  - ~/code/garden\ntags:\n  - outdoor\n---\n";

        assert_eq!(
            repos(content),
            vec![PathBuf::from("/home/someone/code/garden")]
        );
    }

    #[test]
    fn a_note_without_frontmatter_has_no_repos() {
        assert!(repos("# Garden\n\n- [ ] Water the plants\n").is_empty());
    }

    #[test]
    fn a_bullet_after_the_frontmatter_is_not_a_repo() {
        assert!(repos("---\ntitle: Garden\n---\nrepos:\n  - ~/code/garden\n").is_empty());
    }

    #[test]
    fn finds_the_folder_of_the_project_that_owns_a_repository() {
        let projects = vec![
            Project {
                note: PathBuf::from("Projects/Garden/Garden.md"),
                repos: vec![PathBuf::from("/srv/garden")],
            },
            Project {
                note: PathBuf::from("Projects/Kitchen/Kitchen.md"),
                repos: vec![PathBuf::from("/srv/kitchen")],
            },
        ];

        assert_eq!(
            folder_for(&projects, Path::new("/srv/kitchen")),
            Some(PathBuf::from("Projects/Kitchen"))
        );
        assert_eq!(folder_for(&projects, Path::new("/srv/other")), None);
    }
}
