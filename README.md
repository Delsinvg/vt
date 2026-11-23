# vt

Todos in your Markdown notes, from the terminal.

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

`vt` is written in Rust. It is built and tested on macOS and should work on Linux. It needs `git` on your `PATH` for project detection.

With Rust installed ([rustup.rs](https://rustup.rs)):

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
hook_instruction = "..."           # what Claude Code does with the project note
```

The `VAULT` environment variable overrides `vault`, which is handy for a quick test: `VAULT=~/test-notes vt todo`. Folders starting with a dot, such as `.obsidian` and `.trash`, are always skipped.

## Your notes

### Todos

A todo is a Markdown checkbox. Indented lines below it are its details and travel with it when you move it.

```markdown
- [ ] Water the plants 📅 2026-11-27
	- every morning, not the cactus
- [x] Buy seeds ✅ 2026-11-20
```

| Part | Meaning |
|---|---|
| `- [ ]` / `- [x]` | open / done (`[X]` works too) |
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
| `vt recap [period]` | What you checked off; `--project` to narrow it down |
| `vt inbox [text]` | Add a todo, or show the inbox without text |
| `vt tick [search]` | Check off a todo; without a search term you pick from a list |
| `vt due [search] [@date]` | Set, change or clear (`@none`) a due date; asks for the date when you leave it out |
| `vt move [search] [+project]` | Move a todo, with its details, into a project |
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

### Dates

Dates start with `@` and work in English and Dutch. Capitals, dashes and spaces are ignored, so `@Next-Week` and `@volgende week` work too.

| English | Dutch | Means |
|---|---|---|
| `today`, `tomorrow` | `vandaag`, `morgen`, `overmorgen` | that day |
| `monday` … `sunday` | `maandag` … `zondag` | the next one after today |
| `thisweek`, `nextweek` | `dezeweek`, `volgendeweek` | the end of that week (`week_ends_on`) |
| `thismonth`, `nextmonth` | `dezemaand`, `volgendemaand` | the last day of that month |
| `25/11`, `2026-11-25` | | that date; day/month means the next time it comes |
| `none` | `geen` | no due date (`vt due` only) |

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
cargo clippy --all-targets    # lints
cargo fmt                     # formatting
```

| Module | Responsibility |
|---|---|
| `todo` | parse and render one todo line |
| `vault` | find notes, read todos, write changes |
| `markdown` | move blocks and insert under a section, on plain lines |
| `dates` | dates and periods, with the words per language in `dates/vocabulary` |
| `project`, `git` | link a repository or worktree to its project note |
| `links`, `output` | short link names, colors, clickable links |
| `select`, `picker` | matching with typos, interactive lists |
| `config` | `~/.config/vt/config.toml` |
| `commands/*` | one file per kind of command |

Tests use temporary folders and repositories with made-up notes; they never touch your own.
