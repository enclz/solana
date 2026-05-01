## Context

Greenfield repo. SPECIFICATION.md is the source of truth for PDAs, seeds, sizes, and error names. Backend (separate codebase) will mirror PDA seeds + error codes — any drift breaks the agent REST API contract. This change establishes the foundation that all future on-chain work depends on.

## Goals / Non-Goals

**Goals:**
- Buildable Anchor workspace; `anchor build` green from clean checkout.
- All three account structs declared exactly per spec (field names, types, sizes).
- Errors enum that maps 1:1 to backend REST error codes.
- Test infra ready for both LiteSVM unit tests and Anchor mocha integration tests.
- Toolchain pinned so CI + every dev hits identical builds.

**Non-Goals:**
- Any instruction handler logic (split into `add-owner-instructions` + `add-execute-transfer`).
- Backend Node.js code.
- Devnet deploy (deferred to `add-devnet-deploy-pipeline`).
- IDL publishing.

## Decisions

**Anchor 0.30+ over raw `solana-program` / Pinocchio.**
Rationale: spec already prescribes Anchor; PDA + ATA macros cut boilerplate; IDL auto-generated for backend integration. Pinocchio considered but adds friction for a hackathon timeline w/o material CU savings at current scope.

**Single program crate (`programs/enclz/`), not multi-program workspace.**
All on-chain logic is one cohesive policy program. Splitting would force CPI-only paths between trivially related instructions.

**LiteSVM for unit tests + Anchor mocha for integration.**
LiteSVM gives sub-ms execution and deterministic `Clock` for TTL/daily-reset tests. Mocha+test-validator catches real CPI + ATA wiring issues. Surfpool reserved for later mainnet-fork swap tests.

**Pin `rust-toolchain.toml` to a Solana-compatible nightly.**
Avoids "works on my machine" between contributors and CI. Anchor 0.30 is sensitive to rustc version.

**`INIT_SPACE` derived via `#[derive(InitSpace)]`, not hand-counted.**
Eliminates a class of "account too small" bugs and adapts if a field grows.

**Errors live in a single `errors.rs` enum, names mirror backend codes verbatim** (`WhitelistViolation`, `WhitelistExpired`, `WhitelistAmountExhausted`, `DailyLimitExceeded`, `PerTxLimitExceeded`, `HourlyCapExceeded`, `NonceMismatch`, `Unauthorized`, `InvalidAmount`, `InvalidTtl`).
Why: backend translates Anchor error → REST `error` field by name match. Drift = silent miscoding. `InvalidTtl` is separate from `InvalidAmount` — TTL validation failures must map to a distinct REST error code. `InvalidAddress` is omitted: `Pubkey` type validation is enforced by the ABI layer, making an explicit on-chain error unreachable.

**`AgentWallet` and `WhitelistEntry` store their canonical PDA `bump: u8`.**
Saves one `find_program_address` call (= ~1500 CU) on every `execute_transfer`. Set at `init` time; always passed via `seeds::program` or manual signer seeds thereafter.

## Risks / Trade-offs

- [Anchor version churn] → Pin exact patch in `Cargo.toml`; lockfile committed.
- [Default-limit constants drift from backend templates] → Constants live in `constants.rs` w/ comment pointing to SPECIFICATION.md §Templates; CI grep test asserts numbers match docs.
- [`INIT_SPACE` underestimate after later field add] → Always re-run unit test asserting `Account::LEN == expected` after struct edits.
- [Toolchain pin too aggressive may block contributors] → Use `rustup` toolchain file w/ `channel` only; let `rustup` auto-install.

## Migration Plan

N/A — greenfield. No rollback needed; `git revert` removes the workspace cleanly.

## Open Questions

- Final program ID generation: defer until first devnet deploy (placeholder `11111...` until then).
- Whether to ship `cargo-deny` config now or in hardening change → defer to `add-devnet-deploy-pipeline`.
