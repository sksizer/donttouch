# State Machine

donttouch uses an enum-based state machine architecture internally. Understanding the states helps explain why certain commands are available or blocked in certain situations.

## States

```
Uninitialized ──(init)──→ ToInit → Initializing → EndInit → [OfferHooks] → [OfferInject] → Done
                              ↓ (error)
                            Error → End

Enabled ←──(enable)──→ Disabled
   │                      │
   ├── lock               ├── unlock (outside)
   ├── check              ├── disable (outside) 
   ├── status             ├── status
   ├── inject             ├── inject
   ├── why                ├── why
   └── disable (outside)  └── enable
```

## State Resolution

State is **derived from the filesystem**, not stored separately. When you run any command, donttouch:

1. Checks if `.donttouch.toml` exists → if not, state is `Uninitialized`
2. Reads the `enabled` field → `Enabled` or `Disabled`
3. Detects context → `Plain` or `Git { has_husky, hooks_installed }`
4. Discovers files matching patterns and their permission state

## Transition Rules

| Current State | Command | Result |
|--------------|---------|--------|
| Uninitialized | `init` | → ToInit → Initializing → EndInit |
| Uninitialized | anything else | Error: "Run init first" |
| Enabled | `lock` | Lock files (idempotent) |
| Enabled | `disable` | → Disabled (unlocks files) |
| Enabled | `check` | Verify permissions + staged files |
| Disabled | `lock` | Error: "Enable first" |
| Disabled | `enable` | → Enabled (locks files) |
| Disabled | `check` | Skipped |
| Disabled | `check-push` | **Blocked** (can't push while disabled) |

## Context

Context describes the environment and affects behavior without changing states:

```rust
enum Context {
    Plain,                                    // No git
    Git { has_husky: bool, hooks_installed: bool },  // Git repo
}
```

Context influences:
- **init**: Whether to offer hook installation
- **check**: Whether to also check staged files
- **status**: What info to display
- **remove**: Whether to clean up hooks
