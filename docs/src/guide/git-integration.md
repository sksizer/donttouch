# Git Integration

donttouch detects git repos automatically and offers enhanced protection through hooks.

## Hooks

Two hooks are available:

### Pre-commit

Runs `donttouch check` before each commit. Blocks the commit if:
- Any protected file has been staged for commit
- Any protected file is not read-only

### Pre-push

Runs `donttouch check-push` before each push. Blocks the push if:
- Protection is disabled (`enabled = false`)

This prevents you from pushing code while protection is turned off.

## Installing Hooks

### Via `init`

The interactive `donttouch init` wizard offers to install hooks automatically.

### Husky Support

If you use [Husky](https://typicode.github.io/husky/), donttouch detects the `.husky/` directory and installs into your existing Husky hooks rather than overwriting `.git/hooks/`.

### Manual

You can add to your git hooks manually:

```bash
# .git/hooks/pre-commit (or .husky/pre-commit)
donttouch check

# .git/hooks/pre-push (or .husky/pre-push)
donttouch check-push
```

## Plain Directory Mode

Use `--ignoregit` to skip git detection entirely:

```bash
donttouch init --ignoregit
```

This forces plain directory mode even inside a git repo — no hooks, no staged file checking.

## Hook Cleanup

`donttouch remove` cleanly removes hook entries without destroying other hook content. It only removes the lines it added.
