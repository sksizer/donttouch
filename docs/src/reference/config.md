# Config File Reference

## Location

`.donttouch.toml` in the project root.

## Full Example

```toml
[protect]
enabled = true
patterns = [
    "*.toml",
    "Cargo.lock",
    "migrations/**",
    ".env",
    ".env.*",
    "README.md",
    "LICENSE",
    "docker-compose.yml",
]
```

## Schema

### `[protect]`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | `bool` | No | `true` | Whether protection is active |
| `patterns` | `string[]` | Yes | `[]` | Glob patterns relative to project root |

## Notes

- The config file itself is always protected when locked
- Patterns use standard glob syntax
- Paths are relative to the directory containing `.donttouch.toml`
