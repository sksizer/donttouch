# Locking & Unlocking

## How Locking Works

`donttouch lock` sets filesystem permissions to read-only on all files matching your patterns and marks protection as enabled in the config. Execute bits are preserved (so scripts stay executable).

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

No flags needed. Locks all files matching patterns in `.donttouch.toml` and sets `enabled = true`.

## Unlock

```bash
# Must be run from OUTSIDE the project directory
cd ..
donttouch unlock ./my-project
```

Restores write permissions on all protected files, the config file, and sets `enabled = false`. This means git hooks (pre-push) will block pushes until you re-lock — preventing you from accidentally pushing with protection turned off.

### Why Outside-Only?

AI coding agents execute commands from within your project directory. By requiring `unlock` to be called from outside, agents physically cannot bypass protection — even if they try to run the command, the canonical path check will reject it.

This also prevents symlink and path traversal tricks (`../project`, `/proc/self/cwd`, etc.) thanks to `std::fs::canonicalize()`.

## Typical Workflow

```bash
# Unlock from outside the project
cd ..
donttouch unlock ./my-project

# Make your changes
cd my-project
vim config.toml

# Re-lock when done
donttouch lock
```
