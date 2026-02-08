## Why?

AI coding agents are powerful but sometimes overeager. They reformat your configs, "fix" intentional code, and touch files that shouldn't change casually.

**donttouch** lets you draw a hard line.

## Three Layers of Defense

1. **Filesystem permissions** — `chmod` makes files read-only. Hard enforcement — agents get "permission denied."
2. **Git hooks** — Pre-commit blocks staging protected files. Pre-push blocks pushes when unlocked.
3. **Agent instructions** — Injects rules into `CLAUDE.md`, `.cursor/rules/`, `codex.md`, and `.github/copilot-instructions.md`.

## Quick Start

```bash
cargo install donttouch
cd my-project
donttouch init
```

The interactive wizard creates your config, locks files, installs git hooks, and injects agent instructions.

## Configuration

Create `.donttouch.toml` in your project root:

```toml
[protect]
enabled = true
patterns = [
    "*.toml",
    "Cargo.lock",
    "migrations/**",
    ".env",
]
```

## Commands

| Command | Description |
|---------|-------------|
| `donttouch init` | Interactive setup wizard |
| `donttouch lock` | Enable protection + make files read-only |
| `donttouch unlock <path>` | Disable protection + restore write* |
| `donttouch check` | Verify protection (CI-friendly) |
| `donttouch status` | Show current state |
| `donttouch inject` | Add agent instructions |
| `donttouch remove <path>` | Full uninstall* |

*Must be run from **outside** the project directory.*

## The Outside-Directory Rule

This is the key security feature. `unlock` and `remove` must be called from outside the target project directory. Since AI agents execute from within your project, they physically cannot bypass protection.

Symlink and path traversal tricks are blocked via canonical path resolution.

## Works With

- **Claude Code** (CLAUDE.md)
- **Cursor** (.cursor/rules/)
- **GitHub Copilot** (.github/copilot-instructions.md)
- **Codex** (codex.md)
- **Any git workflow** (pre-commit + pre-push hooks)
- **Husky** (auto-detected)
- **Plain directories** (no git required)
