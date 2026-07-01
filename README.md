# skillsdash

<img width="3840" height="2100" alt="image" src="https://github.com/user-attachments/assets/15d2b9a9-4dc2-45e5-a16c-58cff7676243" />

A cross-platform terminal UI for managing AI skills across the **Claude** (`~/.claude/skills`) and **Agents** (`~/.agents/skills`) providers — plus the skills of your current project (`./.claude/skills`, `./.agents/skills`).

Built with [ratatui](https://ratatui.rs) and [tui-textarea](https://github.com/rhysd/tui-textarea). Vim keybindings throughout. Colors follow your terminal theme.

## What it does

- Lists every skill in one place, merged by name, with badges showing which provider and scope holds it. A `*` on a badge means the skill also exists in the current project.
- View a skill's frontmatter and a rendered preview of its `SKILL.md`.
- Create, edit (built-in vim editor), and delete skills.
- Edit frontmatter (name / description) in an in-app form.
- Share a skill to a provider or scope it is missing from — your choice of **copy** or **symlink**.
- **Browse the [skills.sh](https://skills.sh) marketplace** — search, preview a skill's `SKILL.md`, and install it into any provider/scope. Downloads run on a background thread so the UI never blocks.

## Install & run

```sh
cargo run --release
```

Run it from the directory whose project skills you want to manage.

## Keys

### List
| key | action |
| --- | --- |
| `j` / `k` | move down / up |
| `g g` / `G` | top / bottom |
| `ctrl-d` / `ctrl-u` | half-page down / up |
| `/` | filter (type to narrow, `esc` clears, `n` / `N` cycle) |
| `tab` | cycle scope filter: all → global → project |
| `enter` / `l` | open detail |
| `a` | create new skill |
| `e` | edit `SKILL.md` body |
| `f` | edit frontmatter |
| `s` | share to another provider/scope |
| `m` | open the skills.sh marketplace |
| `x` / `D` | delete (choose which instances) |
| `r` | reload from disk |
| `?` | help |
| `q` | quit |

### Marketplace (skills.sh)
| key | action |
| --- | --- |
| type + `enter` | search |
| `j` / `k` | move through results |
| `enter` / `l` | load & preview the skill's `SKILL.md` |
| `i` | install (pick provider/scope; confirms before overwriting) |
| `/` | edit the search query again |
| `esc` / `q` | back to your local skills |

Content is fetched from the source GitHub repo via its public API (no auth). Unauthenticated GitHub allows 60 requests/hour, which is ample for interactive browsing; if you hit the limit the app tells you plainly.

### Detail
`j` / `k` scroll · `ctrl-d` / `ctrl-u` half-page · `e` edit · `f` frontmatter · `s` share · `x` delete · `h` / `esc` back

### Editor (vim-style)
`i` / `a` / `o` insert · `esc` normal · `h j k l` move · `w` / `b` word · `0` / `$` line ends · `g g` / `G` top / bottom · `dd` cut line · `yy` yank · `p` paste · `x` delete char · `u` undo · `ctrl-r` redo · `:w` save · `:q` quit · `:wq` save and quit

## How skills are stored

Each skill is a directory containing a `SKILL.md` with YAML frontmatter:

```markdown
---
name: my-skill
description: what it does and when to use it
---

# My Skill

Body content...
```

Editing frontmatter in the app preserves any extra keys (`license`, `metadata`, etc.) untouched.

## Cross-platform notes

- Works on macOS, Linux, and Windows.
- Symlink sharing on Windows may require Developer Mode or elevated rights; if it fails, use copy instead.

## Development

```sh
cargo test      # unit + integration + headless render tests
cargo clippy    # lints
```
