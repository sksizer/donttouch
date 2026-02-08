# Quick Start

## 1. Initialize

Navigate to your project directory and run:

```bash
donttouch init
```

This will:
- Create a `.donttouch.toml` config file
- Prompt you for file patterns to protect
- Ask if you want to lock the files immediately
- In git repos: offer to install pre-commit and pre-push hooks
- Offer to inject instructions into AI agent config files

## 2. Example Session

```
$ cd my-project
$ donttouch init

✅ Created .donttouch.toml

Add file patterns to protect (glob syntax, one per line).
Examples: .env, secrets/**, docker-compose.prod.yml
Press Enter on an empty line when done.

pattern> .env
   ✅ Added: .env
pattern> .env.*
   ✅ Added: .env.*
pattern> secrets/**
   ✅ Added: secrets/**
pattern>

📝 Saved 3 pattern(s) to .donttouch.toml

Lock protected files now? [Y/n] y
   🔒 ./.env
   🔒 ./.env.prod
   🔒 ./secrets/api.key
   🔒 .donttouch.toml

✅ Locked 4 file(s).

Install git hooks (pre-commit + pre-push)? [Y/n] y
   ✅ Installed pre-commit hook.
   ✅ Installed pre-push hook.
✅ Git hooks installed.

Add agent instructions to coding agent config files? [Y/n] y
   📝 Injected into CLAUDE.md
   📝 Created .cursor/rules/donttouch.mdc

✅ Injected into 2 file(s).
```

## 3. Check Status

```bash
$ donttouch status

🔒 Protection: enabled
📁 Context: git repository
🪝 Hooks: installed

Patterns:
   .env
   .env.*
   secrets/**

Protected files:
   🔒 read-only  ./.env
   🔒 read-only  ./.env.prod
   🔒 read-only  ./secrets/api.key
```

## 4. What Happens When an Agent Tries

```bash
$ echo "HACK" >> .env
bash: .env: Permission denied
```

The file is read-only. The agent physically cannot modify it.

## 5. When You Need to Edit

From **outside** the project directory:

```bash
$ cd ..
$ donttouch disable ./my-project
   🔓 Unlocked 3 file(s).
🔓 Protection disabled.
   ⚠️  You must run 'donttouch enable' before you can push.
```

Make your changes, then re-enable:

```bash
$ cd my-project
$ donttouch enable
   🔒 Locked 3 file(s).
✅ Protection enabled.
```
