# Security Model

donttouch provides **defense in depth** against unauthorized file modifications, with multiple layers that complement each other.

## Layers

### Layer 1: File Permissions (Hard)

Protected files are made read-only at the OS level. No process — human or AI — can write to them without first changing permissions.

**Bypasses**: `chmod u+w` (requires being the file owner or root)

### Layer 2: Git Hooks (Hard, Git Only)

Pre-commit and pre-push hooks enforce protection at git boundaries:
- Pre-commit: blocks commits with protected file changes
- Pre-push: blocks pushes when protection is disabled

**Bypasses**: `git commit --no-verify` / `git push --no-verify`

### Layer 3: Agent Instructions (Soft)

Instructions injected into agent config files tell AI tools not to modify protected files. This is a hint, not enforcement.

**Bypasses**: Agent ignores the instruction

### Layer 4: Outside-Only Operations (Structural)

`disable`, `unlock`, and `remove` can only be run from outside the project directory. This is enforced via canonical path comparison.

**Bypasses**: None from inside the directory (symlinks and `../..` are resolved)

### Layer 5: Config Self-Protection

The `.donttouch.toml` file is itself locked. An agent can't modify the patterns or disable protection by editing the config.

**Bypasses**: Same as Layer 1 (need to change permissions first, which requires Layer 4)

## Threat Model

| Threat | Mitigation |
|--------|-----------|
| Agent writes to protected file | Layer 1 (permissions) |
| Agent stages protected file | Layer 2 (pre-commit hook) |
| Agent disables protection | Layer 4 (outside-only) |
| Agent edits config patterns | Layer 5 (config locked) |
| Agent ignores instructions | Layers 1-2 still enforce |
| Agent uses `--no-verify` | Layer 1 still enforces; CI should also check |
| Human forgets to re-enable | Layer 2 (pre-push blocks) |

## What donttouch Does NOT Protect Against

- **Root access**: A process running as root can change any permissions
- **File owner with intent**: If you `chmod u+w .env`, you can edit it — that's by design
- **CI without donttouch**: If your CI pipeline doesn't run `donttouch check`, the git hook layer is the last defense
- **Copying the file**: An agent could read a protected file and create a copy with a different name

## Recommended Setup

For maximum protection:

1. `donttouch init` — config + lock + hooks + agent instructions
2. Add `donttouch check` to your CI pipeline
3. Never run agents as root
4. Review PRs for new files that look like copies of protected files
