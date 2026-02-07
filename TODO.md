# donttouch — TODO

## v0.1 — Core

- [x] Project scaffolding (Rust + clap CLI)
- [x] `.donttouch.toml` config format (TOML with `[protect]` section)
- [ ] Parse `.donttouch.toml` and load glob patterns
- [ ] `donttouch check` — compare staged files (`git diff --cached --name-only`) against patterns, exit non-zero on violations
- [ ] `donttouch init` — install pre-commit hook into `.git/hooks/` (standalone mode)
- [ ] `donttouch status` — show protected files and any with uncommitted changes
- [ ] Basic error messages with file list on violation

## v0.2 — Enable/Disable + Pre-Push Enforcement

- [ ] `donttouch disable` — sets `enabled = false` in `.donttouch.toml`, pre-commit hook skips checking
- [ ] `donttouch enable` — sets `enabled = true` (default)
- [ ] Pre-push hook — always checks regardless of `enabled` flag. Scans all commits being pushed for protected file changes
- [ ] `donttouch init` installs both pre-commit AND pre-push hooks
- [ ] Clear messaging: "pre-commit checking disabled, but push enforcement is always active"

## v0.3 — Husky Integration

- [ ] Detect existing Husky setup (`.husky/` dir)
- [ ] `donttouch init --husky` — plug into Husky's pre-commit instead of installing standalone hook
- [ ] Graceful fallback: works with or without Husky

## v0.4 — File Permissions Layer

- [ ] `donttouch lock` — `chmod a-w` on all protected files (prevents writes before commit stage)
- [ ] `donttouch unlock` — restore write permissions (for intentional human edits)
- [ ] `donttouch allow -- <command>` — temporarily unlock, run command, re-lock

## v0.5 — Agent Instruction Injection

- [ ] `donttouch inject` — detect agent config files in repo and add "do not modify" instructions
  - [ ] `CLAUDE.md` (Claude Code)
  - [ ] `AGENTS.md` (multi-agent standard)
  - [ ] `.cursor/rules/*.mdc` (Cursor)
  - [ ] `codex.md` / `.codex/` (OpenAI Codex CLI)
  - [ ] `.github/copilot-instructions.md` (GitHub Copilot)
- [ ] Idempotent — re-running doesn't duplicate instructions
- [ ] `donttouch inject --dry-run` — preview what would be added

## v0.6 — GitHub Action

- [ ] `donttouch ci` command — designed for CI, checks all changed files in PR/push
- [ ] Publish reusable GitHub Action (`uses: sksizer/donttouch-action@v1`)
- [ ] Catches `--no-verify` bypasses at the PR level
- [ ] Clear annotations on failing files

## v0.7 — npm Wrapper

- [ ] npm package (`npx donttouch init`) that downloads the correct platform binary
- [ ] Publish to npm
- [ ] Support: macOS (arm64, x64), Linux (x64, arm64), Windows (x64)

## Future Ideas

- [ ] `[protect.delete]` vs `[protect.modify]` — separate patterns for delete-only vs any-change protection
- [ ] `.cursorignore` auto-sync — keep `.cursorignore` in sync with `.donttouch.toml` patterns
- [ ] Watch mode — filesystem watcher that warns immediately on protected file modification
- [ ] `donttouch why <file>` — show which pattern protects a given file
- [ ] Pre-push hook option (in addition to pre-commit)
- [ ] Monorepo support — `.donttouch.toml` at subdirectory level
