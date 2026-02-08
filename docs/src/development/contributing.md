# Contributing

## Building

```bash
git clone https://github.com/sksizer/donttouch
cd donttouch
cargo build
```

## Running Tests

```bash
cargo test
```

## Code Quality

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## CI

Pull requests run checks automatically via GitHub Actions:
- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- Cross-platform build (Linux, macOS, Windows)
