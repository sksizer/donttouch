# Agent Instructions

donttouch can inject "do not modify" instructions into AI coding agent configuration files, providing a **soft guardrail** alongside the hard permission enforcement.

## How It Works

```bash
donttouch inject
```

This scans for known agent config files and appends a one-liner with a marker:

```markdown
<!-- donttouch:managed --> ⚠️ donttouch is active in this project. Files marked read-only are protected — do not modify, rename, or delete them. Run `donttouch status` to see which files are protected.
```

## Supported Agent Files

| File | Agent | Behavior |
|------|-------|----------|
| `CLAUDE.md` | Claude Code | Append (if exists) |
| `AGENTS.md` | Multi-agent standard | Append (if exists) |
| `.cursor/rules/donttouch.mdc` | Cursor | **Created** (Cursor uses per-rule files) |
| `codex.md` | OpenAI Codex CLI | Append (if exists) |
| `.github/copilot-instructions.md` | GitHub Copilot | Append (if exists) |

Most files are only modified if they already exist. The Cursor `.mdc` file is the exception — it's created automatically since Cursor expects individual rule files in `.cursor/rules/`.

## Idempotent

Running `inject` multiple times is safe. It checks for the `<!-- donttouch:managed -->` marker before adding anything:

```bash
$ donttouch inject
   ✅ CLAUDE.md (already has instructions)
   ✅ .cursor/rules/donttouch.mdc (already has instructions)

✅ All agent files already have instructions.
```

## Dry Run

Preview what would be changed without writing:

```bash
$ donttouch inject --dry-run
Dry run:
   📝 Would inject into CLAUDE.md
   📝 Would create .cursor/rules/donttouch.mdc
```

## Cleanup

`donttouch remove` strips all injected instructions:

- Lines containing `<!-- donttouch:managed -->` are removed from existing files
- The `.cursor/rules/donttouch.mdc` file is deleted entirely (since donttouch created it)

## Defense in Depth

Agent instructions alone are unreliable — an agent might ignore them. But combined with file permissions:

1. **Hard layer**: File is read-only → agent physically can't write to it
2. **Soft layer**: Agent instructions tell it not to even try → fewer error messages, better UX

The agent sees the instruction, respects it, and never encounters the permission error. If it ignores the instruction, the permission stops it anyway.
