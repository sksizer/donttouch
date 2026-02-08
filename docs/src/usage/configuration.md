# Configuration

donttouch uses a `.donttouch.toml` file in the root of your project.

## Format

```toml
# donttouch configuration

[protect]
enabled = true
patterns = [
    ".env",
    ".env.*",
    "secrets/**",
    "docker-compose.prod.yml",
]
```

## Fields

### `[protect]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Whether protection is active |
| `patterns` | string[] | `[]` | Glob patterns for files to protect |

### Patterns

Patterns use standard glob syntax:

| Pattern | Matches |
|---------|---------|
| `.env` | Exactly `.env` |
| `.env.*` | `.env.prod`, `.env.staging`, etc. |
| `secrets/**` | Everything under `secrets/` recursively |
| `*.key` | Any file ending in `.key` |
| `config/prod.yml` | Exact path |

## Self-Protection

The `.donttouch.toml` file itself is locked when you run `donttouch lock`. This prevents agents from modifying the protection patterns. It can only be unlocked from outside the project directory.

## Checking Patterns

Use `donttouch why` to see which pattern protects a specific file:

```bash
$ donttouch why .env.prod
.env.prod is protected by:
   • .env.*  (.donttouch.toml:8)
```

The output includes the line number in the config file, which is clickable in most IDE terminals.
