# donttouch

**Protect files from being modified by AI coding agents and accidental changes.**

donttouch is a lightweight CLI tool that prevents AI coding agents (Claude Code, Cursor, Codex, Copilot, etc.) and accidental edits from modifying files you want to keep safe — like `.env` files, secrets, production configs, and migration files.

## How It Works

1. You define glob patterns for files you want to protect
2. donttouch makes those files **read-only** on the filesystem
3. AI agents physically cannot write to them
4. Git hooks catch anything that slips through
5. Agent instruction files tell AI tools not to even try

## Key Features

- 🔒 **File permissions** — Protected files are made read-only (`chmod`). Agents can't write to them.
- 🪝 **Git hooks** — Pre-commit checks for violations. Pre-push blocks if protection is disabled.
- 🤖 **Agent instructions** — Injects "don't modify" instructions into CLAUDE.md, AGENTS.md, Cursor rules, and more.
- 🐶 **Husky support** — Auto-detects Husky and plugs into existing hooks.
- 🔓 **Disable/enable** — Temporarily disable for human edits, but can't push until re-enabled.
- 🛡️ **Self-protecting** — The config file itself is locked. Can only be unlocked from outside the project.
- 📁 **Works anywhere** — Git repos and plain directories alike.

## Why?

AI coding agents are powerful but sometimes modify files they shouldn't — environment files, secrets, production configs, database migrations. Telling them "don't touch that" in a prompt is unreliable. File permissions are enforceable.

donttouch gives you a **hard enforcement layer** (file permissions + git hooks) combined with **soft guidance** (agent instruction injection) for defense in depth.
