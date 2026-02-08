# Architecture

## Single-File Design

donttouch is currently a single-file Rust application (`src/main.rs`). This keeps things simple for a focused CLI tool.

## State Machine

All program flow uses an enum-based state machine:

```rust
enum State {
    Start,
    ToInit,
    Initializing,
    EndInit,
    OfferHooks,
    OfferInject,
    Done(String),
    Error(String),
    End,
}
```

Each state's `run()` method performs its work and returns the next `State`. The main loop drives transitions until `End` is reached.

## Context

The tool detects its operating context:

```rust
enum Context {
    Plain,
    Git {
        has_husky: bool,
        hooks_installed: bool,
    },
}
```

Git context enables hooks and staged-file checking. Plain context uses filesystem permissions only.

## Key Design Decisions

1. **Filesystem-first** — chmod is the primary enforcement, git is layered on top
2. **Outside-directory rule** — `canonicalize()` prevents agents from bypassing protection
3. **Config self-protection** — `.donttouch.toml` is always locked with protected files
4. **Idempotent inject** — Marker comments prevent duplicate agent instructions
5. **Husky-aware** — Detects and integrates with existing Husky setups

## Dependencies

- `clap` — CLI argument parsing
- `glob` — File pattern matching
- `toml` / `serde` — Config file parsing
- Standard library for filesystem operations
