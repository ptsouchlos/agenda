# Installation

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Optionally, [just](https://github.com/casey/just) for convenience commands

## Build from Source

```bash
git clone https://github.com/ptsouchlos/agenda
cd agenda
cargo build --release
```

Or with `just`:

```bash
just build
just install
```

## Verify Installation

```bash
agenda --version
```

## Quick Start

Initialize a default config file:

```bash
agenda init
```

Then set up your preferred [provider](providers/morgen.md) and run:

```bash
agenda
```
