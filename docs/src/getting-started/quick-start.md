# Quick Start

## 1. Initialize

```bash
cd your-project
donttouch init
```

This starts an interactive wizard that:
- Creates `.donttouch.toml` with your file patterns
- Optionally locks files immediately
- Offers to install git hooks (if in a git repo)
- Offers to inject agent instructions

## 2. Protect Files

Define patterns in `.donttouch.toml`:

```toml
[protect]
enabled = true
patterns = [
    "*.toml",
    "migrations/**",
    "README.md",
]
```

Then lock them:

```bash
donttouch lock
```

## 3. Verify

```bash
donttouch status
```

Shows all protected files, their lock state, git hook status, and context.

## 4. Check Protection

```bash
donttouch check
```

Returns exit code 0 if all files are properly locked, non-zero otherwise. Use in CI or git hooks.

## Working With Protected Files

When *you* need to edit a protected file, unlock from **outside** the project:

```bash
cd ..
donttouch unlock ./your-project
```

Make your changes, then re-lock:

```bash
cd your-project
donttouch lock
```

The outside-directory requirement is the key security feature — AI agents running inside your project directory cannot unlock files.
