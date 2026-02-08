# How It Works

donttouch uses three layers of defense to protect files from AI coding agents.

## Layer 1: Filesystem Permissions

The primary mechanism. `donttouch lock` removes write bits from protected files using `chmod`. This is a hard enforcement — any process (including AI agents) will get a "permission denied" error when trying to write.

Execute bits are preserved so scripts remain runnable.

## Layer 2: Git Hooks

In git repositories, donttouch installs hooks:

- **Pre-commit**: Runs `donttouch check` to verify no protected files are staged and all are read-only
- **Pre-push**: Runs `donttouch check-push` to block pushes when protection is disabled

Hooks integrate with Husky if present.

## Layer 3: Agent Instructions

`donttouch inject` writes rules into agent configuration files. This is a soft enforcement — agents that respect their instruction files will avoid protected files even before hitting the permission wall.

## The Outside-Directory Rule

The critical security property: `unlock` and `remove` must be called from **outside** the project directory. Since AI agents execute from within the project, they cannot bypass protection.

This is enforced via `std::fs::canonicalize()` on both the current working directory and the target path, preventing symlink tricks and path traversal.

## State Machine Architecture

Internally, donttouch uses an enum-based state machine for all program flow. Each command is a sequence of state transitions:

```
Start → ToInit → Initializing → EndInit → OfferHooks → OfferInject → Done → End
```

This makes the flow explicit, testable, and easy to extend.
