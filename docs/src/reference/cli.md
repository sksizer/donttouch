# CLI Reference

## `donttouch init`

Initialize donttouch in the current directory.

```
donttouch init [--ignoregit]
```

Interactive flow:
1. Creates `.donttouch.toml`
2. Prompts for glob patterns
3. Offers to lock files
4. (Git) Offers to install hooks
5. Offers to inject agent instructions

## `donttouch status`

Show current protection state.

```
donttouch status [--ignoregit]
```

Displays: enabled/disabled state, context (plain/git/husky), hook status, patterns, and all matching files with their lock state.

## `donttouch lock`

Make all protected files read-only.

```
donttouch lock
```

Idempotent — safe to run multiple times. Also locks `.donttouch.toml`. Only works when protection is enabled.

## `donttouch unlock <target>`

Restore write permissions on protected files.

```
donttouch unlock <target>
```

**Must be run from outside the target directory.** Also unlocks `.donttouch.toml`.

## `donttouch check`

Verify all protected files are read-only.

```
donttouch check
```

Exit code 0 if all files are locked, 1 if any are writable. In git repos, also checks for staged protected files. Used by the pre-commit hook.

## `donttouch check-push`

Check if protection is enabled (used by pre-push hook).

```
donttouch check-push
```

Blocks (exit 1) if protection is disabled. Only meaningful in git repos.

## `donttouch enable`

Re-enable protection and lock all files.

```
donttouch enable
```

Sets `enabled = true` in config, locks all protected files and the config file.

## `donttouch disable <target>`

Disable protection and unlock all files.

```
donttouch disable <target>
```

**Must be run from outside the target directory.** Sets `enabled = false`, unlocks all files. Push will be blocked until re-enabled.

## `donttouch inject`

Add instructions to AI agent config files.

```
donttouch inject [--dry-run]
```

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview changes without writing |

## `donttouch why <file>`

Show which pattern(s) protect a given file.

```
donttouch why <file>
```

Output includes the line number in `.donttouch.toml` (clickable in IDE terminals).

## `donttouch remove <target>`

Completely uninstall donttouch from a directory.

```
donttouch remove <target>
```

**Must be run from outside the target directory.** Unlocks all files, removes config, cleans up hooks, removes agent instructions.

## Global Flags

| Flag | Description |
|------|-------------|
| `--ignoregit` | Ignore git integration |
| `-h, --help` | Print help |
| `-V, --version` | Print version |
