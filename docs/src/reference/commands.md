# CLI Commands

## `donttouch init`

Interactive setup wizard. Creates `.donttouch.toml`, optionally locks files, installs hooks, and injects agent instructions.

**Flags:**
- `--ignoregit` — Force plain directory mode (skip git detection)

## `donttouch lock`

Set protected files to read-only. Also locks `.donttouch.toml`.

## `donttouch unlock <target>`

Restore write permissions on protected files. **Must be run from outside the target directory.**

**Arguments:**
- `target` — Path to the project directory

## `donttouch enable`

Set `enabled = true` in config and lock files. Run from inside the project.

## `donttouch disable <target>`

Set `enabled = false` and unlock files. **Must be run from outside the target directory.**

**Arguments:**
- `target` — Path to the project directory

## `donttouch check`

Verify all protected files are read-only. In git context, also checks that no protected files are staged.

**Exit codes:**
- `0` — All good
- `1` — Violation found

## `donttouch check-push`

Verify protection is enabled. Used in pre-push hooks.

**Exit codes:**
- `0` — Protection enabled
- `1` — Protection disabled

## `donttouch status`

Display current state: patterns, matched files, lock status, context (git/plain), and hook status.

## `donttouch inject`

Inject protection instructions into agent config files.

**Flags:**
- `--dry-run` — Preview what would be written without making changes

## `donttouch remove <target>`

Full cleanup: unlock files, remove config, uninstall hooks, strip agent instructions.

**Must be run from outside the target directory.**

**Arguments:**
- `target` — Path to the project directory
