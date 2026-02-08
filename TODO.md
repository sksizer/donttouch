# donttouch — TODO

## v0.1 — Core

- [x] Project scaffolding (Rust + clap CLI)
- [x] `.donttouch.toml` config format (TOML with `[protect]` section)
- [x] Parse `.donttouch.toml` and load glob patterns
- [x] `donttouch check` — verify protected files are read-only, exit non-zero on violations
- [x] `donttouch init` — interactive setup (create config, prompt patterns, offer lock)
- [x] `donttouch status` — show protected files, lock state, context info
- [x] `donttouch lock` — make protected files + config read-only (preserves execute bits)
- [x] `donttouch unlock <target>` — restore write permissions (must run from outside directory)
- [x] `donttouch remove <target>` — uninstall donttouch (must run from outside directory)
- [x] `.donttouch.toml` itself is locked/unlocked with protected files
- [x] Enum-based state machine architecture
- [x] Delete detection (ACMRD filter)
- [x] Symlink and `../..` traversal protection via canonical paths

## v0.2 — Enable/Disable + Pre-Push Enforcement

- [x] `donttouch disable <target>` — unlocks files, sets `enabled = false` (must run from outside)
- [x] `donttouch enable` — relocks files, sets `enabled = true`
- [x] Pre-push hook blocks push when protection is disabled
- [x] `donttouch check-push` command for pre-push hook

## v0.3 — Git + Husky Integration

- [x] `Context` enum: `Plain` | `Git { has_husky, hooks_installed }`
- [x] Auto-detect git repo and Husky presence
- [x] `--ignoregit` flag to force plain directory mode
- [x] `donttouch init` offers hook installation in git context
- [x] Husky detection: installs into `.husky/` if present, `.git/hooks/` otherwise
- [x] `donttouch check` also detects staged protected files in git context
- [x] `donttouch status` shows context and hook status
- [x] `donttouch remove` cleans up hooks (preserves other hook content)

## v0.4 — Agent Instruction Injection

- [x] `donttouch inject` — add instruction line to agent config files
- [x] `donttouch inject --dry-run` — preview without writing
- [x] Offered during `init` flow (after hooks)
- [x] Targets: CLAUDE.md, AGENTS.md, .cursor/rules/donttouch.mdc, codex.md, .github/copilot-instructions.md
- [x] Creates `.cursor/rules/donttouch.mdc` (Cursor per-rule file format)
- [x] Idempotent — `<!-- donttouch:managed -->` marker prevents duplicates
- [x] `donttouch remove` strips injected lines from all agent files

## v0.5 — `donttouch allow` (Temporary Bypass)

- [ ] `donttouch allow -- <command>` — temporarily unlock, run command, re-lock
- [ ] Scoped bypass without disabling protection entirely

## v0.6 — GitHub Action

- [ ] `donttouch ci` command — designed for CI, checks all changed files in PR/push
- [ ] Publish reusable GitHub Action (`uses: sksizer/donttouch-action@v1`)
- [ ] Catches `--no-verify` bypasses at the PR level
- [ ] Clear annotations on failing files

## v0.7 — npm Wrapper / Distribution

- [ ] npm package (`npx donttouch init`) that downloads the correct platform binary
- [ ] Publish to npm
- [ ] Support: macOS (arm64, x64), Linux (x64, arm64), Windows (x64)
- [ ] `cargo install donttouch` support (publish to crates.io)

## Future Ideas

- [ ] `donttouch add <pattern>` / `donttouch remove-pattern <pattern>` — CLI pattern management
- [ ] `.cursorignore` auto-sync — keep `.cursorignore` in sync with `.donttouch.toml` patterns
- [ ] Watch mode — filesystem watcher that warns immediately on protected file modification
- [ ] `donttouch why <file>` — show which pattern protects a given file
- [ ] Monorepo support — `.donttouch.toml` at subdirectory level
