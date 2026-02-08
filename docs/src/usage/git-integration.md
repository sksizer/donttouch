# Git Integration

donttouch automatically detects git repositories and adds git-specific protections.

## Context Detection

When you run any donttouch command, it checks for:

1. **`.git/` directory** — Is this a git repo?
2. **`.husky/` directory** — Is Husky installed?
3. **Existing hooks** — Are donttouch hooks already installed?

Use `--ignoregit` to force plain directory mode:

```bash
donttouch --ignoregit init
```

## Hooks

### Pre-Commit Hook

Runs `donttouch check` before each commit:
- Verifies all protected files are read-only
- In git context, also checks for **staged** protected files
- Blocks the commit if violations are found

### Pre-Push Hook

Runs `donttouch check-push` before each push:
- If protection is **disabled**, the push is **blocked**
- Forces you to run `donttouch enable` before code leaves your machine
- This is the safety net — you can disable locally for convenience, but can't push without re-enabling

## Husky Support

If donttouch detects a `.husky/` directory, it installs hooks there instead of `.git/hooks/`:

```
🐶 Husky detected.
Install donttouch hooks into Husky? [Y/n] y
   ✅ Added donttouch to existing pre-commit hook.
   ✅ Installed pre-push hook.
```

When appending to existing Husky hooks, donttouch adds its block without disturbing other hook content (like `lint-staged`).

## Staged File Detection

In git repos, `donttouch check` goes beyond permission checking:

```bash
$ donttouch check
🚫 donttouch check failed!

Permission violations (files are writable):
   • ./.env

Staged file violations (protected files in git staging area):
   • .env

Run 'donttouch lock' to fix permission issues.
```

## The Disable/Push Flow

1. You `donttouch disable ./project` from outside — files unlocked
2. You work freely, commit whatever you need
3. `git push` → **blocked** ("Protection is disabled, re-enable first")
4. `donttouch enable` → files relocked
5. `git push` → ✅ allowed

## Cleanup

`donttouch remove` cleans up hooks. If the hook file only contained donttouch, it's deleted. If it had other content, only the donttouch block is removed:

```bash
$ donttouch remove ./project
   ✅ Removed donttouch from pre-commit hook.  # Other hooks preserved
   🗑️  Removed pre-push hook (was only donttouch).
```
