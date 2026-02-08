# Installation

## From Source (Cargo)

If you have Rust installed:

```bash
cargo install donttouch
```

## From Source (Build)

```bash
git clone https://github.com/sksizer/donttouch.git
cd donttouch
cargo build --release
```

The binary will be at `target/release/donttouch`. Copy it to a directory in your `PATH`:

```bash
cp target/release/donttouch ~/.local/bin/
```

## Verify Installation

```bash
donttouch --version
```
