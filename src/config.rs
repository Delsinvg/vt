use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Weekday;
use serde::Deserialize;

use crate::dates::{DEFAULT_MONTH_END, DEFAULT_WEEK_END, weekday_named};
use crate::opener::{FILE_PLACEHOLDER, LINE_PLACEHOLDER, Opener};
use crate::recurrence::DayOfMonth;

const DEFAULT_INBOX: &str = "Inbox.md";
const DEFAULT_SECTION: &str = "Todo";
const DEFAULT_AGENT_SECTION: &str = "Agent";
const VAULT_VARIABLE: &str = "VAULT";
const HOME_PREFIX: &str = "~/";
const LAST_DAY: &str = "last";
const DEFAULT_ARCHIVE_AFTER_DAYS: u32 = 30;

#[derive(Debug)]
pub struct Layout {
    pub inbox: PathBuf,
    pub section_heading: String,
    pub agent_heading: String,
}

#[derive(Debug)]
pub struct Settings {
    pub vault: Option<PathBuf>,
    pub layout: Layout,
    pub ignored: Vec<PathBuf>,
    pub week_ends_on: Weekday,
    pub month_ends_on: DayOfMonth,
    pub opener: Opener,
    pub archive_after_days: u32,
    pub hook_instruction: Option<String>,
    pub source: PathBuf,
}

impl Settings {
    pub fn vault_root(&self) -> io::Result<PathBuf> {
        self.vault.clone().ok_or_else(|| {
            io::Error::other(format!(
                "no vault configured. Add vault = \"~/Documents/notes\" to {}, or export VAULT",
                self.source.display()
            ))
        })
    }
}

#[derive(Debug)]
pub struct ConfigError {
    path: PathBuf,
    reason: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.reason)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    vault: Option<String>,
    inbox: Option<String>,
    section: Option<String>,
    agent_section: Option<String>,
    ignore: Option<Vec<String>>,
    week_ends_on: Option<String>,
    month_ends_on: Option<MonthEnd>,
    open_with: Option<String>,
    archive_after_days: Option<i64>,
    hook_instruction: Option<String>,
}

#[derive(Deserialize)]
#[serde(
    untagged,
    expecting = "month_ends_on: use a day of the month from 1 to 31, or \"last\""
)]
enum MonthEnd {
    Day(i64),
    Word(String),
}

impl MonthEnd {
    fn day_of_month(&self) -> Option<DayOfMonth> {
        match self {
            Self::Day(day) => DayOfMonth::fixed(u32::try_from(*day).ok()?),
            Self::Word(word) if word == LAST_DAY => Some(DayOfMonth::Last),
            Self::Word(_) => None,
        }
    }
}

impl fmt::Display for MonthEnd {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Day(day) => write!(formatter, "{day}"),
            Self::Word(word) => write!(formatter, "{word}"),
        }
    }
}

pub fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

pub fn load() -> Result<Settings, ConfigError> {
    let home = home();
    load_from(&config_path(&home), env::var_os(VAULT_VARIABLE), &home)
}

fn config_path(home: &Path) -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("vt/config.toml")
}

fn load_from(
    path: &Path,
    vault_variable: Option<OsString>,
    home: &Path,
) -> Result<Settings, ConfigError> {
    let file = read_config(path)?;

    let vault = vault_variable
        .map(PathBuf::from)
        .or_else(|| file.vault.map(|vault| expand_home(&vault, home)));
    let section = file.section.unwrap_or_else(|| DEFAULT_SECTION.to_string());
    let agent_section = file
        .agent_section
        .unwrap_or_else(|| DEFAULT_AGENT_SECTION.to_string());
    let week_ends_on = match file.week_ends_on {
        Some(day) => weekday_named(&day).ok_or_else(|| ConfigError {
            path: path.to_path_buf(),
            reason: format!("week_ends_on: '{day}' is not a day of the week"),
        })?,
        None => DEFAULT_WEEK_END,
    };
    let month_ends_on = match file.month_ends_on {
        Some(end) => end.day_of_month().ok_or_else(|| ConfigError {
            path: path.to_path_buf(),
            reason: format!(
                "month_ends_on: '{end}' is not a day of the month; use 1 to 31 or \"{LAST_DAY}\""
            ),
        })?,
        None => DEFAULT_MONTH_END,
    };
    let opener = match file.open_with {
        Some(template) => Opener::from_template(&template).ok_or_else(|| ConfigError {
            path: path.to_path_buf(),
            reason: format!(
                "open_with: '{template}' does not say where the note goes; add {FILE_PLACEHOLDER}, e.g. \"nvim +{LINE_PLACEHOLDER} {FILE_PLACEHOLDER}\""
            ),
        })?,
        None => Opener::Obsidian,
    };
    let archive_after_days = match file.archive_after_days {
        Some(days) => u32::try_from(days).map_err(|_| ConfigError {
            path: path.to_path_buf(),
            reason: format!("archive_after_days: '{days}' is not a number of days; use 0 or more"),
        })?,
        None => DEFAULT_ARCHIVE_AFTER_DAYS,
    };
    let ignored = file
        .ignore
        .unwrap_or_default()
        .iter()
        .map(|folder| PathBuf::from(folder.trim_end_matches('/')))
        .collect();

    Ok(Settings {
        vault,
        layout: Layout {
            inbox: PathBuf::from(file.inbox.unwrap_or_else(|| DEFAULT_INBOX.to_string())),
            section_heading: format!("## {section}"),
            agent_heading: format!("## {agent_section}"),
        },
        ignored,
        week_ends_on,
        month_ends_on,
        opener,
        archive_after_days,
        hook_instruction: file.hook_instruction,
        source: path.to_path_buf(),
    })
}

fn read_config(path: &Path) -> Result<ConfigFile, ConfigError> {
    let failure = |reason: String| ConfigError {
        path: path.to_path_buf(),
        reason,
    };

    match fs::read_to_string(path) {
        Ok(content) => {
            toml::from_str(&content).map_err(|error| failure(error.message().to_string()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ConfigFile::default()),
        Err(error) => Err(failure(error.to_string())),
    }
}

fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix(HOME_PREFIX) {
        Some(inside_home) => home.join(inside_home),
        None => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const HOME: &str = "/home/someone";

    fn load_with(content: Option<&str>, vault_variable: Option<&str>) -> Result<Settings, String> {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("config.toml");
        if let Some(content) = content {
            fs::write(&path, content).unwrap();
        }
        load_from(&path, vault_variable.map(OsString::from), Path::new(HOME))
            .map_err(|error| error.to_string())
    }

    #[test]
    fn reads_every_setting_from_the_file() {
        let settings = load_with(
            Some("vault = \"~/notes\"\ninbox = \"Capture.md\"\nsection = \"Algemeen\"\n"),
            None,
        )
        .unwrap();

        assert_eq!(settings.vault, Some(PathBuf::from("/home/someone/notes")));
        assert_eq!(settings.layout.inbox, PathBuf::from("Capture.md"));
        assert_eq!(settings.layout.section_heading, "## Algemeen");
    }

    #[test]
    fn the_agent_section_is_called_agent_unless_configured() {
        assert_eq!(
            load_with(None, None).unwrap().layout.agent_heading,
            "## Agent"
        );
        assert_eq!(
            load_with(Some("agent_section = \"Voor Claude\"\n"), None)
                .unwrap()
                .layout
                .agent_heading,
            "## Voor Claude"
        );
    }

    #[test]
    fn uses_defaults_without_a_file() {
        let settings = load_with(None, None).unwrap();

        assert_eq!(settings.vault, None);
        assert_eq!(settings.layout.inbox, PathBuf::from("Inbox.md"));
        assert_eq!(settings.layout.section_heading, "## Todo");
    }

    #[test]
    fn the_vault_variable_wins_over_the_file() {
        let settings = load_with(Some("vault = \"~/notes\"\n"), Some("/srv/other-notes")).unwrap();

        assert_eq!(settings.vault, Some(PathBuf::from("/srv/other-notes")));
    }

    #[test]
    fn reads_ignored_folders_week_end_and_hook_instruction() {
        let settings = load_with(
            Some("ignore = [\"Templates\", \"Archive/\"]\nweek_ends_on = \"zondag\"\nhook_instruction = \"Summarise the note.\"\n"),
            None,
        )
        .unwrap();

        assert_eq!(
            settings.ignored,
            vec![PathBuf::from("Templates"), PathBuf::from("Archive")]
        );
        assert_eq!(settings.week_ends_on, Weekday::Sun);
        assert_eq!(
            settings.hook_instruction.as_deref(),
            Some("Summarise the note.")
        );
    }

    #[test]
    fn the_week_ends_on_friday_by_default() {
        let settings = load_with(None, None).unwrap();

        assert_eq!(settings.week_ends_on, Weekday::Fri);
        assert!(settings.ignored.is_empty());
        assert_eq!(settings.hook_instruction, None);
    }

    #[test]
    fn a_month_ends_on_its_last_day_unless_configured() {
        assert_eq!(
            load_with(None, None).unwrap().month_ends_on,
            DayOfMonth::Last
        );
        assert_eq!(
            load_with(Some("month_ends_on = 25\n"), None)
                .unwrap()
                .month_ends_on,
            DayOfMonth::Fixed(25)
        );
        assert_eq!(
            load_with(Some("month_ends_on = \"last\"\n"), None)
                .unwrap()
                .month_ends_on,
            DayOfMonth::Last
        );
    }

    #[test]
    fn a_month_end_that_is_not_a_day_of_the_month_is_an_error() {
        for (value, shown) in [
            ("0", "0"),
            ("32", "32"),
            ("-1", "-1"),
            ("\"first\"", "first"),
        ] {
            let error = load_with(Some(&format!("month_ends_on = {value}\n")), None).unwrap_err();

            assert!(
                error.ends_with(&format!(
                    "month_ends_on: '{shown}' is not a day of the month; use 1 to 31 or \"last\""
                )),
                "{error}"
            );
        }
    }

    #[test]
    fn a_month_end_that_is_neither_a_number_nor_a_word_is_an_error() {
        let error = load_with(Some("month_ends_on = 2.5\n"), None).unwrap_err();

        assert!(error.ends_with("month_ends_on: use a day of the month from 1 to 31, or \"last\""));
    }

    #[test]
    fn opens_in_obsidian_unless_a_command_is_configured() {
        assert_eq!(load_with(None, None).unwrap().opener, Opener::Obsidian);
        assert_eq!(
            load_with(Some("open_with = \"zed {file}:{line}\"\n"), None)
                .unwrap()
                .opener,
            Opener::from_template("zed {file}:{line}").unwrap()
        );
    }

    #[test]
    fn a_command_that_does_not_name_the_file_is_an_error() {
        let error = load_with(Some("open_with = \"zed\"\n"), None).unwrap_err();

        assert!(error.ends_with(
            "open_with: 'zed' does not say where the note goes; add {file}, e.g. \"nvim +{line} {file}\""
        ));
    }

    #[test]
    fn archives_after_thirty_days_unless_configured() {
        assert_eq!(load_with(None, None).unwrap().archive_after_days, 30);
        assert_eq!(
            load_with(Some("archive_after_days = 7\n"), None)
                .unwrap()
                .archive_after_days,
            7
        );
    }

    #[test]
    fn a_negative_number_of_days_to_archive_after_is_an_error() {
        let error = load_with(Some("archive_after_days = -3\n"), None).unwrap_err();

        assert!(error.ends_with("archive_after_days: '-3' is not a number of days; use 0 or more"));
    }

    #[test]
    fn an_unknown_week_end_is_an_error() {
        let error = load_with(Some("week_ends_on = \"someday\"\n"), None).unwrap_err();

        assert!(error.ends_with("week_ends_on: 'someday' is not a day of the week"));
    }

    #[test]
    fn an_unknown_setting_is_an_error() {
        let error = load_with(Some("sectoin = \"Todo\"\n"), None).unwrap_err();

        assert!(error.contains("config.toml"));
        assert!(error.contains("unknown field `sectoin`"));
    }

    #[test]
    fn invalid_toml_is_an_error() {
        assert!(load_with(Some("vault = \n"), None).is_err());
    }

    #[test]
    fn a_missing_vault_explains_how_to_configure_it() {
        let settings = load_with(None, None).unwrap();

        let error = settings.vault_root().unwrap_err().to_string();

        assert!(error.starts_with("no vault configured. Add vault = "));
        assert!(error.contains("config.toml"));
    }

    #[test]
    fn an_absolute_vault_is_used_as_is() {
        let settings = load_with(Some("vault = \"/srv/notes\"\n"), None).unwrap();

        assert_eq!(settings.vault, Some(PathBuf::from("/srv/notes")));
    }

    #[test]
    fn a_config_that_cannot_be_read_is_an_error() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("config.toml");
        fs::create_dir(&path).unwrap();

        let error = load_from(&path, None, Path::new(HOME)).unwrap_err();

        assert!(error.to_string().starts_with(&path.display().to_string()));
    }
}
