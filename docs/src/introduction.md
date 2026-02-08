# donttouch

**Protect your files from AI coding agents.**

`donttouch` is a CLI tool that prevents AI coding assistants (Claude Code, Cursor, Copilot, Codex, etc.) from modifying files you want to keep safe. It uses a layered defense:

1. **Filesystem permissions** — Makes files read-only via `chmod`
2. **Git hooks** — Blocks commits and pushes that touch protected files
3. **Agent instructions** — Injects rules into agent config files (CLAUDE.md, .cursorrules, etc.)

## Why?

AI coding agents are powerful but sometimes overeager. They might:

- Reformat your carefully crafted config files
- "Fix" code you intentionally wrote a certain way
- Modify documentation you maintain by hand
- Touch infrastructure files that shouldn't change casually

`donttouch` gives you a simple way to draw a line: *these files are off-limits*.

## Key Features

- **Works everywhere** — Git repos and plain directories
- **Pattern-based** — Protect files with glob patterns (`*.toml`, `migrations/**`)
- **Agent-aware** — Injects instructions into Claude, Cursor, Copilot, and Codex config files
- **Git-integrated** — Pre-commit and pre-push hooks with optional Husky support
- **Safe by design** — `unlock` and `disable` must be run from outside the project directory, so agents inside the repo can't bypass protection

## Quick Example

```bash
cd my-project
donttouch init        # Interactive setup
donttouch lock        # Make protected files read-only
donttouch inject      # Add rules to agent config files
donttouch status      # See what's protected
```

To edit protected files (from outside the project):

```bash
cd ..
donttouch unlock ./my-project
# make your changes
cd my-project
donttouch lock
```

Note: `unlock` also disables protection, so pre-push hooks will block until you `lock` again. This prevents accidentally pushing with protection off.
