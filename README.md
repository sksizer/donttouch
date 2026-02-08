# 🚫 donttouch

**(sort of) guards files from AI coding agents**

[![Crates.io](https://img.shields.io/crates/v/donttouch.svg)](https://crates.io/crates/donttouch)
[![Docs](https://img.shields.io/badge/docs-latest-blue.svg)](https://sksizer.github.io/donttouch/)
[![MIT License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

donttouch puts a lightweight guardrail around files, to keep AI coding assistants from modifying them.

Think of it as a sign and velvet rope around the files. It should be a clear signal to any AI agent: "Do not change this file."

...but they can get around it. This should not be replacing code reviews or human oversight. It's just an experiment as an extra layer of protection and perhaps more importantly a stronger way to signal the invariants around files.





`donttouch` prevents AI assistants (Claude Code, Cursor, Copilot, Codex) from modifying files you want to keep safe — using filesystem permissions, git hooks, and agent instruction injection.

## Why?

AI coding agents are powerful but sometimes overeager. They reformat configs, "fix" intentional code, and touch files that shouldn't change. `donttouch` lets you draw a hard line.

## Install

```bash
cargo install donttouch
```

Or build from source:

```bash
git clone https://github.com/sksizer/donttouch
cd donttouch
cargo install --path .
```

## Quick Start

```bash
cd my-project

# Interactive setup — creates config, offers to lock files, install hooks, inject agent rules
donttouch init

# Or do it manually:
donttouch lock          # Make protected files read-only
donttouch inject        # Add rules to agent config files
donttouch status        # See what's protected
donttouch check         # Verify protection (use in CI)
```

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
    "README.md",
]
```

## How It Works

Three layers of defense:

1. **Filesystem permissions** — `chmod` makes files read-only. Hard enforcement.
2. **Git hooks** — Pre-commit blocks staging protected files. Pre-push blocks pushes when protection is disabled.
3. **Agent instructions** — Injects rules into `CLAUDE.md`, `.cursor/rules/`, `codex.md`, and `.github/copilot-instructions.md`.

## Key Commands

| Command | Description |
|---------|-------------|
| `donttouch init` | Interactive setup wizard |
| `donttouch lock` | Enable protection + make files read-only |
| `donttouch unlock <path>` | Disable protection + restore write permissions* |
| `donttouch check` | Verify protection (CI-friendly) |
| `donttouch status` | Show current state |
| `donttouch inject` | Add agent instructions |
| `donttouch remove <path>` | Full uninstall* |

*\*Must be run from **outside** the project directory — this is the key security feature. Agents running inside your repo can't bypass protection.*

## The Outside-Directory Rule

`unlock`, `disable`, and `remove` require you to run them from outside the target project. Since AI agents execute from within your project, they physically cannot disable protection. Symlink and path traversal tricks are blocked via canonical path resolution.

## Git Integration

- Auto-detects git repos and [Husky](https://typicode.github.io/husky/)
- Installs pre-commit and pre-push hooks
- Use `--ignoregit` to force plain directory mode

## Documentation

Full docs: [donttouch book](https://sksizer.github.io/donttouch/) (mdbook)

## License

MIT
