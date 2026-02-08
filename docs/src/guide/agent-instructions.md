# Agent Instructions

donttouch can inject rules directly into AI agent configuration files, telling agents not to modify protected files.

## Supported Agents

| Agent | File | Behavior |
|-------|------|----------|
| Claude Code | `CLAUDE.md` | Appends if file exists |
| OpenClaw / Custom | `AGENTS.md` | Appends if file exists |
| Cursor | `.cursor/rules/donttouch.mdc` | Creates file |
| Codex | `codex.md` | Appends if file exists |
| GitHub Copilot | `.github/copilot-instructions.md` | Appends if file exists |

## Usage

```bash
donttouch inject
```

Preview without writing:

```bash
donttouch inject --dry-run
```

## What Gets Injected

Each file gets a block wrapped in markers:

```markdown
<!-- donttouch:managed -->
## Protected Files (donttouch)

The following files are protected by donttouch and must not be modified:
- *.toml
- migrations/**

Do not edit, move, rename, or delete these files.
<!-- /donttouch:managed -->
```

## Idempotency

Running `inject` multiple times is safe. The `<!-- donttouch:managed -->` markers are checked — if the block already exists, it's updated in place rather than duplicated.

## Cleanup

`donttouch remove` strips the injected blocks from all agent files.
