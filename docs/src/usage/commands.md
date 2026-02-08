# Commands

## Overview

| Command | Description | Requires outside? |
|---------|-------------|:-----------------:|
| `init` | Initialize donttouch in a directory | No |
| `status` | Show protection state and files | No |
| `lock` | Make protected files read-only | No |
| `unlock <target>` | Restore write permissions | ✅ Yes |
| `check` | Verify all files are locked | No |
| `check-push` | Block push if disabled (hook use) | No |
| `enable` | Re-enable protection + lock files | No |
| `disable <target>` | Disable protection + unlock files | ✅ Yes |
| `inject` | Add agent instructions to config files | No |
| `why <file>` | Show which pattern protects a file | No |
| `remove <target>` | Completely uninstall donttouch | ✅ Yes |

## Commands Requiring Outside Access

`unlock`, `disable`, and `remove` must be run from **outside** the target directory. This prevents AI coding agents working inside the project from disabling protection.

```bash
# From inside the project — blocked
$ donttouch disable .
🚫 This command must be run from OUTSIDE the target directory.

# From outside — works
$ cd ..
$ donttouch disable ./my-project
🔓 Protection disabled.
```

Path resolution uses `canonicalize()` to prevent symlink and `../..` traversal tricks.

## Global Flags

| Flag | Description |
|------|-------------|
| `--ignoregit` | Treat directory as plain (ignore `.git/`) |
| `--version` | Show version |
| `--help` | Show help |
