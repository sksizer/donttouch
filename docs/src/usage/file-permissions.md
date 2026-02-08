# File Permissions

donttouch's primary protection mechanism is filesystem permissions.

## How It Works

When you run `donttouch lock`:

1. All files matching your patterns have their **write bits removed** (`chmod a-w`)
2. The `.donttouch.toml` config file is also made read-only
3. Execute bits are **preserved** — a script that was `755` becomes `555`, not `444`

When you unlock or disable:

1. Owner write bit is **restored** (`chmod u+w`)
2. Other permission bits remain unchanged

## Why Permissions?

Telling an AI agent "don't modify `.env`" in a prompt is a **soft guardrail** — the agent might ignore it, forget it, or misunderstand it. File permissions are a **hard guardrail** — the operating system enforces them regardless of what any process wants to do.

```bash
# Agent tries to write
$ echo "HACK" >> .env
bash: .env: Permission denied
```

No prompt engineering required.

## Lock and Unlock

```bash
# Lock all protected files
$ donttouch lock
   🔒 ./.env
   🔒 ./.env.prod
   🔒 .donttouch.toml

✅ Locked 3 file(s).

# Lock is idempotent — safe to run multiple times
$ donttouch lock
   (3 already read-only)
✅ All protected files are already read-only.

# New files matching patterns get caught on next lock
$ echo "x" > .env.staging
$ donttouch lock
   🔒 ./.env.staging

✅ Locked 1 file(s).
   (3 already read-only)
```

## Unlock Requires Outside Access

```bash
# Must run from outside the project
$ cd /tmp
$ donttouch unlock /path/to/project
   🔓 /path/to/project/.env
   🔓 /path/to/project/.donttouch.toml

✅ Unlocked 2 file(s).
```

This prevents an AI agent working inside the project from unlocking its own restrictions.
