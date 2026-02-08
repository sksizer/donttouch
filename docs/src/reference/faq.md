# FAQ

## Can an agent bypass donttouch?

An agent running inside the project directory **cannot**:
- Write to protected files (read-only permissions)
- Disable protection (`disable` requires being outside the directory)
- Modify the config (`.donttouch.toml` is locked)
- Use symlinks or `../..` tricks (paths are canonicalized)

An agent **could** theoretically:
- Use `chmod` to change permissions (but most agent sandboxes restrict this)
- Use `git commit --no-verify` to skip hooks (but permissions still block writes)

## Does it work on Windows?

Yes. donttouch uses Rust's cross-platform permission APIs. On Windows, it uses the `readonly` file attribute. The CI pipeline builds and tests on Windows.

## Can I use it without git?

Yes. donttouch works in any directory. Git integration (hooks, staged file checking) is automatic when a `.git/` directory is detected, but all core functionality (permissions, lock/unlock, disable/enable) works without git.

Use `--ignoregit` to explicitly disable git integration even in a git repo.

## Why can't I disable from inside the project?

This is intentional. AI coding agents typically operate from inside the project directory. By requiring `disable` and `unlock` to be run from outside, we prevent an agent from disabling its own restrictions.

## What happens if I add new files?

New files matching your patterns won't be locked automatically. Run `donttouch lock` again to catch them — it's idempotent and will only lock the new files.

The pre-commit hook will also catch new staged files matching protected patterns in git repos.

## Can I have different patterns for different directories?

Not yet. Currently, `.donttouch.toml` applies to the entire directory tree from where it's located. Monorepo support is planned for a future release.

## How do I see why a file is protected?

```bash
donttouch why <filename>
```

This shows which pattern(s) match and the line number in `.donttouch.toml`.

## Does it conflict with Husky?

No. donttouch auto-detects Husky and appends to existing hooks rather than replacing them. Your `lint-staged`, `commitlint`, or other Husky hooks continue to work.

## What about `.gitignore`d files?

donttouch protects files based on glob patterns against the filesystem, not the git index. If a file matches a pattern, it's protected regardless of `.gitignore` status.
