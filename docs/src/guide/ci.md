# CI / GitHub Actions

## Using `donttouch check` in CI

Add a step to your CI pipeline to verify protected files haven't been modified:

```yaml
- name: Check protected files
  run: |
    cargo install donttouch
    donttouch check
```

`donttouch check` exits with code 0 if all protected files are read-only and no staged changes affect them. Non-zero means a violation.

## Planned: `donttouch ci`

A dedicated `donttouch ci` command is planned that will:
- Compare the PR diff against protected patterns
- Fail the check if any protected file was modified
- Work without needing file permissions (useful in CI containers)

Stay tuned — this is on the roadmap for v0.6.
