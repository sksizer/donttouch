# Locking & Unlocking

## How Locking Works

`donttouch lock` sets filesystem permissions to read-only on all files matching your patterns. Execute bits are preserved (so scripts stay executable).

```bash
# Before lock
-rw-r--r--  config.toml
-rwxr-xr-x  deploy.sh

# After lock  
-r--r--r--  config.toml
-r-xr-xr-x  deploy.sh
```

The `.donttouch.toml` config file is also locked to prevent agents from modifying protection rules.

## Lock

```bash
donttouch lock
```

No flags needed. Locks all files matching patterns in `.donttouch.toml`.

## Unlock

```bash
# Must be run from OUTSIDE the project directory
cd ..
donttouch unlock ./my-project
```

Restores write permissions on all protected files and the config file.

### Why Outside-Only?

AI coding agents execute commands from within your project directory. By requiring `unlock` to be called from outside, agents physically cannot bypass protection — even if they try to run the command, the canonical path check will reject it.

This also prevents symlink and path traversal tricks (`../project`, `/proc/self/cwd`, etc.) thanks to `std::fs::canonicalize()`.

## Disable / Enable

`disable` is like `unlock` but also sets `enabled = false` in the config:

```bash
cd ..
donttouch disable ./my-project
```

`enable` re-locks and sets `enabled = true`:

```bash
cd my-project
donttouch enable
```

The difference matters for git hooks — `check-push` blocks pushes when protection is disabled, ensuring you don't accidentally push with protection turned off.
