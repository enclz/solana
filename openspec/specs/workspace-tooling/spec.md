# workspace-tooling Specification

## Purpose
TBD - created by archiving change init-anchor-workspace. Update Purpose after archive.
## Requirements
### Requirement: Anchor + Cargo workspace

The repo SHALL contain a Cargo workspace with a single Anchor program crate at `programs/enclz/` and an `Anchor.toml` configured for `localnet`, `devnet`, and `mainnet-beta` clusters.

#### Scenario: Clean build succeeds
- **WHEN** developer runs `anchor build` on a fresh clone
- **THEN** build completes without warnings-as-errors and produces `target/deploy/enclz.so`

#### Scenario: Clusters configured
- **WHEN** test parses `Anchor.toml`
- **THEN** `[provider]` section references each of the three clusters with placeholder URLs (devnet pointing at QuickNode env var)

### Requirement: Pinned Rust toolchain

The repo SHALL contain a `rust-toolchain.toml` pinning the channel to a Solana-compatible version.

#### Scenario: Toolchain auto-installs
- **WHEN** developer runs any `cargo` command on a clean machine with `rustup`
- **THEN** the pinned toolchain installs automatically and is used for the build

### Requirement: Test framework wiring

The repo SHALL include dev dependencies for `litesvm` + `litesvm-token` (Rust unit tests) and `@coral-xyz/anchor` + `mocha` + `chai` + `ts-node` (TypeScript integration tests).

#### Scenario: Rust unit tests run
- **WHEN** developer runs `cargo test --package enclz`
- **THEN** placeholder unit tests in `programs/enclz/src/` execute via LiteSVM and exit 0

#### Scenario: Anchor integration tests run
- **WHEN** developer runs `anchor test`
- **THEN** mocha discovers `tests/*.spec.ts` and runs them against `solana-test-validator`

### Requirement: Git ignores build artifacts

The `.gitignore` SHALL exclude `target/`, `.anchor/`, `node_modules/`, and `test-ledger/`.

#### Scenario: Build does not pollute git status
- **WHEN** developer runs `anchor build` followed by `git status`
- **THEN** no files under `target/` or `.anchor/` appear as untracked

