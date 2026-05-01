## Why

Repo currently contains only `docs/` — no buildable code. To deliver the on-chain spend-policy enforcement that is Enclz's core value prop, we need an Anchor workspace with the program's account state, error taxonomy, and constants in place before any instruction logic can be written. Locking these primitives first prevents drift between SPECIFICATION.md and code.

## What Changes

- New Cargo workspace at repo root with `programs/enclz/` (Anchor 0.30+).
- `Anchor.toml` configured for devnet via QuickNode RPC + local validator profile.
- Pinned Rust toolchain (`rust-toolchain.toml`).
- Program scaffolding: `lib.rs` entry, module wiring, placeholder `declare_id!`.
- Account state structs (`GroupConfig`, `AgentWallet`, `WhitelistEntry`) with `#[account]` + `INIT_SPACE` matching SPECIFICATION.md exactly.
- Seed-prefix constants and default-limit constants.
- Error enum mirroring REST error codes (so backend can pass through unchanged).
- Dev-deps wired: `litesvm`, `litesvm-token`, `@coral-xyz/anchor`, `mocha`, `chai`, `ts-node`.
- Unit tests asserting PDA derivation matches documented seeds and `INIT_SPACE` is correct.
- `anchor build` passes from a clean checkout.

## Capabilities

### New Capabilities
- `program-state`: PDA struct definitions, seed constants, INIT_SPACE sizing, error enum
- `workspace-tooling`: Anchor + Cargo workspace, toolchain pinning, test framework wiring

### Modified Capabilities
<!-- none — this is the first change -->

## Impact

- Creates `Cargo.toml`, `Anchor.toml`, `rust-toolchain.toml`, `programs/enclz/**`, `tests/**`, `.gitignore` for `target/` + `.anchor/`.
- No backend or frontend code touched.
- Downstream changes (`add-owner-instructions`, `add-execute-transfer`) depend on this scaffold + state module being in place.
