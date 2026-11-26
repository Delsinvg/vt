use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        for folder in ["config", "vault", "outside", "bin"] {
            fs::create_dir(root.path().join(folder)).unwrap();
        }
        Self { root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        fs::canonicalize(self.root.path()).unwrap().join(relative)
    }

    fn note(&self, name: &str, content: &str) {
        fs::write(self.path("vault").join(name), content).unwrap();
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.path("vault").join(name)).unwrap()
    }

    fn command(&self, arguments: &[&str], directory: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vt"));
        command
            .args(arguments)
            .current_dir(directory)
            .env("HOME", self.root.path())
            .env("XDG_CONFIG_HOME", self.path("config"))
            .env("VAULT", self.path("vault"))
            .env("PATH", self.search_path())
            .stdin(Stdio::null());
        command
    }

    fn search_path(&self) -> String {
        format!(
            "{}:{}",
            self.path("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    fn fake_opener(&self) -> PathBuf {
        let opened = self.path("opened");
        let script = format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", opened.display());
        for name in ["open", "xdg-open"] {
            let opener = self.path("bin").join(name);
            fs::write(&opener, &script).unwrap();
            fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
        }
        opened
    }

    fn run_in(&self, arguments: &[&str], directory: &Path) -> Output {
        self.command(arguments, directory).output().unwrap()
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_in(arguments, &self.path("outside"))
    }

    fn vt(&self, arguments: &[&str]) -> String {
        succeeded(self.run(arguments))
    }
}

fn succeeded(output: Output) -> String {
    assert!(
        output.status.success(),
        "vt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn failed(output: Output) -> String {
    assert!(!output.status.success(), "vt should have failed");
    String::from_utf8(output.stderr).unwrap()
}

fn git(directory: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap()
        .status;
    assert!(status.success(), "git {arguments:?} failed");
}

#[test]
fn a_todo_added_to_the_inbox_shows_up_in_every_list() {
    let sandbox = Sandbox::new();

    assert_eq!(
        sandbox.vt(&["inbox", "Call the plumber", "@2099-01-15"]),
        "Added to inbox: Call the plumber (due 2099-01-15)\n"
    );
    assert_eq!(
        sandbox.read("Inbox.md"),
        "- [ ] Call the plumber 📅 2099-01-15\n"
    );
    assert_eq!(
        sandbox.vt(&["inbox"]),
        "Inbox\n  ○ Call the plumber  📅 2099-01-15\n"
    );
    assert_eq!(
        sandbox.vt(&["todo"]),
        "Inbox\n  ○ Call the plumber  📅 2099-01-15\n"
    );
    assert_eq!(
        sandbox.vt(&["agenda"]),
        "2099-01-15           Call the plumber  (Inbox)\n"
    );
}

#[test]
fn a_repeating_todo_comes_back_when_it_is_checked_off() {
    let sandbox = Sandbox::new();

    assert_eq!(
        sandbox.vt(&["inbox", "Pay the rent @2099-01-15 @every-month"]),
        "Added to inbox: Pay the rent (due 2099-01-15, repeats every month on the 15th)\n"
    );
    assert_eq!(
        sandbox.vt(&["tick", "rent"]),
        "Checked off: Pay the rent, again on 2099-02-15\n"
    );

    let inbox = sandbox.read("Inbox.md");
    assert!(inbox.starts_with(
        "- [ ] Pay the rent 🔁 every month on the 15th 📅 2099-02-15\n\
         - [x] Pay the rent 🔁 every month on the 15th 📅 2099-01-15 ✅ "
    ));
    assert!(
        sandbox
            .vt(&["recap", "today"])
            .contains("  Pay the rent  (Inbox)\n")
    );
}

#[test]
fn due_takes_every_at_word_given() {
    let sandbox = Sandbox::new();
    sandbox.note("Inbox.md", "- [ ] Buy milk\n");

    assert_eq!(
        sandbox.vt(&["due", "milk", "@2099-03-06", "@every-week"]),
        "Buy milk: due 2099-03-06, repeats every week\n"
    );
    assert_eq!(
        sandbox.vt(&["due", "milk", "@none"]),
        "Buy milk: no due date\n"
    );
    assert_eq!(sandbox.read("Inbox.md"), "- [ ] Buy milk\n");
}

#[test]
fn a_todo_moves_into_a_project_with_its_details() {
    let sandbox = Sandbox::new();
    sandbox.note("Inbox.md", "- [ ] Buy a hose\n\t- the long one\n");
    sandbox.note("Garden.md", "# Garden\n## Todo\n");

    assert_eq!(
        sandbox.vt(&["move", "hose", "+garden"]),
        "Moved to Garden: Buy a hose\n"
    );
    assert_eq!(
        sandbox.read("Garden.md"),
        "# Garden\n## Todo\n\n- [ ] Buy a hose\n\t- the long one\n"
    );
}

#[test]
fn a_todo_opens_in_obsidian_at_its_heading() {
    let sandbox = Sandbox::new();
    let opened = sandbox.fake_opener();
    sandbox.note("Garden.md", "# Garden\n## Todo\n- [ ] Rake the leaves\n");

    assert_eq!(
        sandbox.vt(&["open", "rake"]),
        "Opened Garden: Rake the leaves\n"
    );
    assert!(
        fs::read_to_string(opened)
            .unwrap()
            .ends_with("%2Fvault%2FGarden.md%23Todo")
    );
}

#[test]
fn a_configured_editor_opens_the_note_at_the_line_of_the_todo() {
    let sandbox = Sandbox::new();
    let opened = sandbox.fake_opener();
    fs::create_dir(sandbox.path("config/vt")).unwrap();
    fs::write(
        sandbox.path("config/vt/config.toml"),
        "open_with = \"open {file}:{line}\"\n",
    )
    .unwrap();
    sandbox.note("Garden.md", "# Garden\n## Todo\n- [ ] Rake the leaves\n");

    sandbox.vt(&["open", "rake"]);

    assert_eq!(
        fs::read_to_string(opened).unwrap(),
        format!("{}:3", sandbox.path("vault/Garden.md").display())
    );
}

#[test]
fn old_checked_off_todos_move_to_an_archive_next_to_their_note() {
    let sandbox = Sandbox::new();
    sandbox.note(
        "Garden.md",
        "# Garden\n## Todo\n- [ ] Rake the leaves\n- [x] Buy seeds ✅ 2000-01-01\n",
    );

    assert_eq!(
        sandbox.vt(&["archive"]),
        "Archived 1 checked-off todo from Garden into Garden archive\n"
    );
    assert_eq!(
        sandbox.read("Garden.md"),
        "# Garden\n## Todo\n- [ ] Rake the leaves\n"
    );
    assert_eq!(
        sandbox.read("Garden archive.md"),
        "# Garden archive\n\n## Todo\n- [x] Buy seeds ✅ 2000-01-01\n"
    );
}

#[test]
fn the_agent_keeps_its_own_todos_apart() {
    let sandbox = Sandbox::new();
    sandbox.note(
        "Garden.md",
        "# Garden\n## Todo\n- [ ] Rake the leaves\n## Agent\n- [ ] Add tests\n",
    );

    assert_eq!(
        sandbox.vt(&["todo"]),
        "Garden\n  Todo\n    ○ Rake the leaves\n"
    );
    assert_eq!(
        sandbox.vt(&["todo", "--agent"]),
        "Garden\n  Agent\n    ○ Add tests\n"
    );
}

#[test]
fn inside_a_project_repository_only_that_project_is_listed() {
    let sandbox = Sandbox::new();
    let repository = sandbox.path("garden");
    fs::create_dir(&repository).unwrap();
    git(&repository, &["init", "--quiet"]);
    fs::create_dir_all(sandbox.path("vault/Garden")).unwrap();
    sandbox.note(
        "Garden/Garden.md",
        &format!(
            "---\nrepos:\n  - {}\n---\n# Garden\n## Todo\n- [ ] Rake the leaves\n",
            repository.display()
        ),
    );
    sandbox.note("Kitchen.md", "# Kitchen\n## Todo\n- [ ] Fix the tap\n");

    let in_repository = |arguments: &[&str]| succeeded(sandbox.run_in(arguments, &repository));

    assert_eq!(
        in_repository(&["todo"]),
        "Garden\n  Todo\n    ○ Rake the leaves\n"
    );
    assert!(in_repository(&["todo", "--all"]).contains("Fix the tap"));
    assert_eq!(
        in_repository(&["todo", "kitchn"]),
        "Kitchen\n  Todo\n    ○ Fix the tap\n"
    );
    assert!(in_repository(&["hook"]).contains("Rake the leaves"));
}

#[test]
fn the_hook_stays_silent_outside_a_repository() {
    let sandbox = Sandbox::new();

    assert_eq!(sandbox.vt(&["hook"]), "");
}

#[test]
fn the_vault_can_come_from_the_config_file_in_the_home_folder() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.path(".config/vt")).unwrap();
    fs::write(
        sandbox.path(".config/vt/config.toml"),
        format!("vault = \"{}\"\n", sandbox.path("vault").display()),
    )
    .unwrap();
    sandbox.note("Inbox.md", "- [ ] Buy milk\n");

    let output = sandbox
        .command(&["todo"], &sandbox.path("outside"))
        .env_remove("VAULT")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap();

    assert_eq!(succeeded(output), "Inbox\n  ○ Buy milk\n");
}

#[test]
fn the_hook_never_fails_even_with_a_broken_config() {
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.path("config/vt")).unwrap();
    fs::write(sandbox.path("config/vt/config.toml"), "vault = \n").unwrap();

    assert_eq!(sandbox.vt(&["hook"]), "");
    assert!(failed(sandbox.run(&["todo"])).contains("config.toml"));
}

#[test]
fn errors_are_reported_on_stderr_with_a_failing_exit_code() {
    let sandbox = Sandbox::new();

    let without_vault = sandbox
        .command(&["todo"], &sandbox.path("outside"))
        .env_remove("VAULT")
        .output()
        .unwrap();
    assert!(failed(without_vault).starts_with("vt: no vault configured."));

    assert!(failed(sandbox.run(&["recap", "someday"])).starts_with("vt: unknown period 'someday'"));
}
