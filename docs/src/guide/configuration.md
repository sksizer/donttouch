# Configuration

donttouch uses a TOML config file: `.donttouch.toml`

## File Format

```toml
[protect]
enabled = true
patterns = [
    "*.toml",
    "Cargo.lock",
    "migrations/**",
    ".env",
    "README.md",
]
```

## Fields

### `[protect]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Whether protection is active |
| `patterns` | string[] | `[]` | Glob patterns for files to protect |

## Patterns

Patterns use standard glob syntax:

| Pattern | Matches |
|---------|---------|
| `*.toml` | All `.toml` files in the root |
| `migrations/**` | Everything under `migrations/` |
| `docker-compose.yml` | Exact file |
| `*.lock` | All lock files |

Patterns are resolved relative to the project root (where `.donttouch.toml` lives).

## Self-Protection

The `.donttouch.toml` file itself is always protected when you run `lock`. This prevents agents from modifying the config to remove patterns.

## Enabled Flag

When `enabled = false`:
- `lock` is a no-op
- `check` passes (no enforcement)
- Pre-push hook blocks (forces you to re-enable before pushing)

Toggle with:
```bash
donttouch enable    # Sets enabled=true, locks files
donttouch disable   # Sets enabled=false, unlocks files (run from outside project)
```
