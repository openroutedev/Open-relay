# OpenRelay Protocol (v0.3)

A modular, privacy-preserving physical routing protocol engineered for peer-to-peer package logistics, escrow collateral bonding, and onion-routed handoffs.

## Workspace Architecture

* `openrelay-crypto`: Ed25519/X25519 identities, BLAKE3 commitments, and HPKE onion layer encryption.
* `openrelay-protocol`: Finite state machine, physical dual-signature handoffs, and SQLite persistence.
* `openrelay-bonding`: Escrow deposit adapters, collateral locking, and diversity route validation.
* `openrelay-label`: Compact QR code generator and printable PDF manifest renderer.
* `openrelay-daemon`: Background node API server managing P2P transport and daemon state.
* `openrelay-cli`: Full-featured administrative command-line interface.

## Quickstart

### Build Workspace
```bash
cargo build --release
```

### Run Tests
```bash
cargo test --workspace
```

### Run Node Daemon
```bash
cargo run -p openrelay-daemon
```
