<p align="center">
  <img src="assets/logo.svg" alt="vt" width="250">
</p>

<p align="center">Todos in your Markdown notes, from the terminal.</p>

## Why vt exists

Working on many projects at once means switching all day: a hotfix here, a review there, a question from a colleague in between. The hard part is rarely knowing *what* to do. It is remembering *where you left off* in each project, and holding on to the things you notice along the way without losing your focus. Todo apps keep tasks away from the notes and code they belong to, and every new app is one more place to check instead of one less.

So `vt` stays small and works with what you already have:

- **Your notes are the database.** Todos are plain Markdown checkboxes in the notes you already keep, right next to their context. Obsidian, git and any editor keep working.
- **Capturing takes seconds.** One command or one hotkey, then straight back to what you were doing.
- **It knows which project you are in.** Inside a repository you see that project's todos; anywhere else, everything.
- **It works with an AI agent.** Claude Code gets the project note when a session starts and keeps its own follow-ups in a separate section, so your list only shows what you have to do yourself.
- **No server, no account, no app.** A single program that reads and writes files.

## What it does

`vt` reads and writes plain `- [ ]` checkboxes in a folder of Markdown notes, such as an Obsidian vault. Capture a todo in a second, see what is open per project, check things off and look back on your week, without leaving the terminal and without a separate todo app. Your notes stay the single source of truth: everything `vt` writes is ordinary Markdown that Obsidian and its Tasks plugin understand.

```
$ vt inbox Review https://gitlab.com/acme/shop/-/merge_requests/12 @friday
Added to inbox: Review shop!12 (due 2026-11-27)

$ vt agenda
2026-11-22  overdue  Fix the tap  (Kitchen)
2026-11-23  today    Call the plumber  (Inbox)
2026-11-27           Review shop!12  (Inbox)

$ vt tick plumber
Checked off: Call the plumber

$ vt todo
Garden
  Spring
    ○ Rake the leaves
    ○ Buy seeds         📅 2026-11-27
  Todo
    ○ Water the plants  📅 2026-11-18
```

## Install

`vt` is built and tested on macOS and should work on Linux. It needs `git` on your `PATH` for project detection.

With [Homebrew](https://brew.sh), on macOS or Linux:

```sh
brew install delsinvg/tap/vt
```

`brew upgrade vt` picks up new releases.

Or build it yourself with Rust ([rustup.rs](https://rustup.rs)):

```sh
cargo install --git https://github.com/Delsinvg/vt
```

Or from a clone:

```sh
git clone https://github.com/Delsinvg/vt && cd vt
cargo install --path .
```

Both put `vt` in `~/.cargo/bin`, which rustup adds to your `PATH`.

## Configure

Tell `vt` where your notes are in `~/.config/vt/config.toml`. Every setting is optional; only `vault` has no default.

```toml
vault = "~/Documents/notes"        # where your notes are
inbox = "Inbox.md"                 # the note new todos go to
section = "Todo"                   # the "## " section projects collect todos in
agent_section = "Agent"            # the "## " section for an AI agent's own todos
ignore = ["Templates", "Archive"]  # folders, relative to the vault, that vt skips
week_ends_on = "friday"            # the day @thisweek and @nextweek point to
month_ends_on = "last"             # the day @thismonth and @nextmonth point to: "last" or 1 to 31
open_with = "zed {file}:{line}"    # how vt open opens a note; Obsidian when left out
archive_after_days = 30            # how long a checked-off todo stays in its note before vt archive moves it
hook_instruction = "..."           # what Claude Code does with the project note
```

The `VAULT` environment variable overrides `vault`, which is handy for a quick test: `VAULT=~/test-notes vt todo`. Folders starting with a dot, such as `.obsidian` and `.trash`, are always skipped.

`vt open` opens a note in Obsidian, at the heading the todo sits under. With `open_with` it runs your editor instead, at the todo's line: `{file}` becomes the note and `{line}` the line number, for example `"nvim +{line} {file}"`, `"zed {file}:{line}"` or `"code --goto {file}:{line}"`. The command runs without a shell, so a note with spaces in its name stays one argument.

## Your notes

### Todos

A todo is a Markdown checkbox. Indented lines below it are its details and travel with it when you move it.

```markdown
- [ ] Water the plants 🔁 every 3 days 📅 2026-11-27
	- every morning, not the cactus
- [x] Buy seeds ✅ 2026-11-20
```

| Part | Meaning |
|---|---|
| `- [ ]` / `- [x]` | open / done (`[X]` works too) |
| `🔁 every 3 days` | it repeats; see [Repeating todos](#repeating-todos) |
| `📅 2026-11-27` | due date |
| `✅ 2026-11-20` | the day it was checked off, written by `vt tick` |
| `[shop!12](https://…)` | a link; `vt` shows the label and makes it clickable in your terminal |

This is the format of the Obsidian [Tasks](https://github.com/obsidian-tasks-group/obsidian-tasks) plugin, so you can keep using its queries next to `vt`.

### Projects

A project is a note with a todo section, by default `## Todo`. `vt move` and `vt inbox … +project` put todos there.

```markdown
---
repos:
  - ~/code/garden
---
# Garden

## Todo
- [ ] Rake the leaves
```

### The agent's own todos

An AI agent such as Claude Code can keep its own follow-ups in the same note, under `## Agent` at the bottom: code to write, review points, work for the next branch. They stay out of your lists, so only what you have to do yourself shows up in `vt todo`, `agenda`, `tick` and `recap`. `vt todo --agent` shows them, and `vt hook` tells the agent that this section is its own.

```markdown
## Agent
### Branch 2
- [ ] Add tests for the watering schedule
```

The optional `repos:` list links the note to one or more git repositories. Inside such a repository, or any of its worktrees, `vt todo` shows only the todos of that project's folder, and `vt hook` hands the note to Claude Code.

## Commands

| Command | What it does |
|---|---|
| `vt todo [project]` | Open todos, grouped per note and heading, with due dates. Inside a project's repository only that project; `--all` for everything, `--agent` for the agent's todos |
| `vt agenda [project]` | Every open todo with a due date, earliest first; overdue in red, today in yellow |
| `vt recap [period]` | What you checked off, archived todos included; `--project` to narrow it down |
| `vt inbox [text]` | Add a todo, or show the inbox without text |
| `vt tick [search]` | Check off a todo; without a search term you pick from a list. A repeating todo comes back with its next date |
| `vt due [search] [@date]` | Set, change or clear (`@none`) a due date or repetition; asks for the date when you leave it out |
| `vt move [search] [+project]` | Move a todo, with its details, into a project |
| `vt archive` | Move todos checked off more than `archive_after_days` ago, with their details, to `<note> archive.md` next to their note |
| `vt open [search]` | Open the note of a todo in Obsidian at its heading, or in your editor at its line (`open_with`) |
| `vt hook` | Print the project note for a Claude Code `SessionStart` hook |

Search terms and project names forgive typos: `vt todo kitchn` finds the Kitchen note. When a search matches several todos, `vt` shows a list to pick from in a terminal, and refuses outside one, so scripts never act on the wrong todo.

### Adding todos

```sh
vt inbox Call the plumber
vt inbox Call the plumber @tomorrow
vt inbox Rake the leaves +garden @nextweek        # straight into a project
vt inbox Review https://github.com/acme/garden/pull/125
```

Links get a short, clickable name:

| Link | Shown as |
|---|---|
| GitLab merge request | `shop!12` |
| GitLab issue, GitHub pull request or issue | `shop#3`, `garden#125` |
| Jira ticket, also from a board link | `SHOP-42` |
| Linear issue | `GAR-7` |
| anything else | `link` |

In zsh, quote a URL that contains `?`, or add `alias vt="noglob vt"` to your `.zshrc`.

### Repeating todos

Give a todo an `@every-…` word, and checking it off puts the next one right above it:

```sh
vt inbox Send the weekly report @every-friday
vt inbox Pay the rent @1/12 @every-month          # a date sets the first time
vt due plants @elke-3-dagen                       # make an existing todo repeat
```

```
$ vt tick report
Checked off: Send the weekly report, again on 2026-12-04
```

```markdown
- [ ] Send the weekly report 🔁 every week 📅 2026-12-04
- [x] Send the weekly report 🔁 every week 📅 2026-11-27 ✅ 2026-11-27
```

- The next date counts from the due date, so a weekly todo stays on its weekday even when you check it off late. Rules that end in `when done`, which you can write in Obsidian, count from the day you checked it off instead.
- Missed times are skipped: the next date is always after today.
- The details below the todo come along, with their checkboxes open again, so a checklist starts fresh every time.
- `vt due … @date` moves the next time and keeps the repetition; `@none` removes both.

`vt` writes and reads the rules of the Obsidian [Tasks](https://publish.obsidian.md/tasks/Getting+Started/Recurring+Tasks) plugin: `every day`, `every 3 days`, `every week`, `every 2 weeks`, `every month on the 1st`, `every month on the last`, `every year`, each optionally followed by `when done`. A rule it does not understand, such as `every monday and thursday`, stays in your note untouched; `vt tick` then only checks the todo off and says so.

### Archiving checked-off todos

Checked-off todos pile up in a note, and a repeating todo leaves one behind every time. `vt archive` moves the ones checked off more than 30 days ago (`archive_after_days`) to an archive next to their note:

```
$ vt archive
Archived 3 checked-off todos from Garden into Garden archive
Dated 1 checked-off todo in Kitchen without a date; they go to the archive in 30 days
```

`Garden/Garden archive.md` keeps the headings the todos sat under, so `## Agent` todos stay the agent's. A todo checked off without a `✅` date, by hand or in Obsidian without the Tasks plugin, first gets today's date. `vt recap` counts an archived todo for the project it came from, and `vt move` never offers an archive as a project. Run it whenever you like: twice in a row, the second run does nothing.

### Dates

Dates start with `@` and work in English and Dutch. Capitals, dashes and spaces are ignored, so `@Next-Week` and `@volgende week` work too.

| English | Dutch | Means |
|---|---|---|
| `today`, `tomorrow` | `vandaag`, `morgen`, `overmorgen` | that day |
| `monday` … `sunday` | `maandag` … `zondag` | the next one after today |
| `thisweek`, `nextweek` | `dezeweek`, `volgendeweek` | the end of that week (`week_ends_on`) |
| `thismonth`, `nextmonth` | `dezemaand`, `volgendemaand` | the end of that month (`month_ends_on`); once that day has passed, a month later |
| `25/11`, `2026-11-25` | | that date; day/month means the next time it comes |
| `every-friday` | `elke-vrijdag` | the next Friday, then every week |
| `every-day`, `every-3-days` | `elke-dag`, `elke-3-dagen` | today, then every (3) days |
| `every-week`, `every-2-weeks` | `elke-week`, `elke-2-weken` | the end of this week, then every (2) weeks |
| `every-month`, `every-3-months` | `elke-maand`, `elke-3-maanden` | the end of this month, then on that day every (3) months |
| `every-year` | `elk-jaar` | the end of this year, then every year |
| `daily`, `weekly`, `monthly`, `yearly` | `dagelijks`, `wekelijks`, `maandelijks`, `jaarlijks` | the same as `every-day`, `every-week`, `every-month`, `every-year` |
| `none` | `geen` | no due date and no repetition |

Next to an `every` word, one plain date sets the first time: `@1/12 @every-month` starts on 1 December and repeats on the 1st. `@every-friday` already says the day, so it takes no date.

### Periods for `vt recap`

An `@` in front is allowed, so `vt recap @lastweek` works too.

| English | Dutch | Covers |
|---|---|---|
| `thisweek` (default) | `dezeweek` | Monday until today |
| `lastweek` | `vorigeweek` | last Monday until Sunday |
| `today`, `yesterday` | `vandaag`, `gisteren` | that day |
| `thismonth`, `lastmonth` | `dezemaand`, `vorigemaand` | the month so far, or the whole previous one |
| `thisyear`, `lastyear` | `ditjaar`, `vorigjaar` | the year so far, or the whole previous one |

## Claude Code

`vt hook` gives Claude Code the project note of the repository you start a session in, so it knows where you left off. Add it to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      { "hooks": [{ "type": "command", "command": "vt hook" }] }
    ]
  }
}
```

The hook prints nothing outside a git repository or without a configured vault, and never fails, so it cannot block a session. In a repository that no project note lists, it asks Claude to offer creating one. Change what Claude is asked to do with `hook_instruction`.

## Development

```sh
cargo test                    # unit and integration tests, no real notes needed
cargo llvm-cov                # test coverage, after: cargo install cargo-llvm-cov
cargo clippy --all-targets    # lints
cargo fmt                     # formatting
```

| Module | Responsibility |
|---|---|
| `todo` | parse and render one todo line |
| `vault` | find notes, read todos, write changes |
| `markdown` | move blocks and insert under a section, on plain lines |
| `dates` | dates, periods and repetitions you type, with the words per language in `dates/vocabulary` |
| `recurrence` | repetition rules in the Tasks format and the next due date |
| `project`, `git` | link a repository or worktree to its project note |
| `links`, `output` | short link names, colors, clickable links |
| `select`, `picker` | matching with typos, interactive lists |
| `config` | `~/.config/vt/config.toml` |
| `opener` | the command that opens a note: Obsidian or the `open_with` editor |
| `archive` | which checked-off todos leave a note, and where they go in its archive |
| `commands/*` | one file per kind of command |

Tests use temporary folders and repositories with made-up notes; they never touch your own.

## License

[MIT](LICENSE)
