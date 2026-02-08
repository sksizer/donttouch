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
- `check` passes (no enforcement)
- Pre-push hook blocks (forces you to re-lock before pushing)

The flag is managed automatically:
- `donttouch lock` sets `enabled = true`
- `donttouch unlock` sets `enabled = false`
