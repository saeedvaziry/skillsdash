# skillsdash

<img width="3840" height="2100" alt="image" src="https://github.com/user-attachments/assets/15d2b9a9-4dc2-45e5-a16c-58cff7676243" />

A cross-platform terminal UI for managing AI skills across the **Claude** (`~/.claude/skills`) and **Agents** (`~/.agents/skills`) providers — plus the skills of your current project (`./.claude/skills`, `./.agents/skills`).

Press `h` for the **harness** view to edit each provider's memory file
(`CLAUDE.md` / `AGENTS.md`, global and project), or `c` for the **commands**
view to edit slash-command files (`~/.claude/commands`, `./.claude/commands`,
and the `.agents` equivalents). In either view you can write one provider's file
and symlink the other to it, so both harnesses share a single source of truth
(`s`).

## Install

### Homebrew (macOS & Linux)

```sh
brew tap saeedvaziry/skillsdash https://github.com/saeedvaziry/skillsdash
brew trust saeedvaziry/skillsdash
brew install skillsdash
```

Homebrew 6.0+ requires third-party taps to be trusted before install; the
`brew trust` step is a one-time confirmation.

### Arch Linux (AUR)

```sh
yay -S skillsdash-bin      # or: paru -S skillsdash-bin
```

### curl (macOS & Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/saeedvaziry/skillsdash/main/install.sh | sh
```

### From source

```sh
cargo install --git https://github.com/saeedvaziry/skillsdash
```

## License

[MIT](LICENSE)
