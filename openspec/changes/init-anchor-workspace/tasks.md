## 1. Workspace scaffold

- [x] 1.1 Run `anchor init enclz --no-git` and move generated tree to repo root layout (`programs/enclz/`, `Anchor.toml`, `tests/`, `migrations/`)
- [x] 1.2 Add root-level `Cargo.toml` workspace declaring `programs/*`
- [x] 1.3 Create `rust-toolchain.toml` pinning channel to Solana-compatible nightly
- [x] 1.4 Configure `Anchor.toml` with `[provider]` blocks for `localnet`, `devnet` (QuickNode env), `mainnet-beta`
- [x] 1.5 Add `.gitignore` entries for `target/`, `.anchor/`, `node_modules/`, `test-ledger/`, `.env`
- [x] 1.6 Add `package.json` with dev deps: `@coral-xyz/anchor`, `mocha`, `chai`, `ts-node`, `@types/mocha`, `@types/chai`, `typescript`
- [x] 1.7 Add Rust dev deps in `programs/enclz/Cargo.toml`: `litesvm`, `litesvm-token`
- [x] 1.8 Verify `anchor build` succeeds on clean checkout (Anchor 1.0.1, anchor-spl 1.0.1, real program ID `67i3uY4gZaidynKa8XbNW569qACSVCebwKnLpNYVtWjj` synced into `lib.rs` + `Anchor.toml`)

## 2. Constants + errors

- [x] 2.1 Create `programs/enclz/src/constants.rs` with `GROUP_SEED`, `WALLET_SEED`, `WHITELIST_SEED`, `DEFAULT_DAILY_LIMIT = 10_000_000`, `DEFAULT_PER_TX_LIMIT = 1_000_000`, `DEFAULT_HOURLY_CAP = 5`, `PROTOCOL_FEE_BPS = 10`
- [x] 2.2 Create `programs/enclz/src/errors.rs` with `EnclzError` enum: `WhitelistViolation`, `WhitelistExpired`, `WhitelistAmountExhausted`, `DailyLimitExceeded`, `PerTxLimitExceeded`, `HourlyCapExceeded`, `NonceMismatch`, `Unauthorized`, `InvalidAmount`, `InvalidTtl`
- [x] 2.3 Wire `mod constants; mod errors;` in `lib.rs`

## 3. Account state

- [x] 3.1 Create `programs/enclz/src/state/mod.rs` re-exporting submodules
- [x] 3.2 Create `state/group_config.rs` with `GroupConfig { owner, backend_operator, protocol_fee_wallet, agent_count }` + `#[account] #[derive(InitSpace)]`
- [x] 3.3 Create `state/agent_wallet.rs` with full struct per SPECIFICATION.md (group, display_name [u8;32], daily_limit, per_tx_limit, hourly_tx_cap, spent_today, tx_count_this_hour, last_spend_reset, last_hour_reset, operator_nonce, bump)
- [x] 3.4 Create `state/whitelist_entry.rs` with full struct (label [u8;32], added_by, entry_type, ttl_expires_at, approved_amount, amount_used, bump) + `EntryType` constants
- [x] 3.5 Wire `mod state;` in `lib.rs`

## 4. Tests

- [x] 4.1 Write Rust unit test asserting each PDA derivation matches `find_program_address` with documented seeds
- [x] 4.2 Write Rust unit test asserting `INIT_SPACE` for each account is sufficient (allocate, write all fields, read back)
- [x] 4.3 Write Rust unit test asserting all required `EnclzError` variants exist + each has stable error code number
- [x] 4.4 Write Rust unit test asserting constants match spec values
- [x] 4.5 Add placeholder `tests/enclz.spec.ts` mocha file that loads the program and asserts it deploys to `solana-test-validator`
- [x] 4.6 Run `cargo test` + `anchor test` — `cargo test` green (14/14); `anchor test --validator legacy` green (2/2 mocha tests). Anchor 1.0 defaults to surfpool; `npm run test:e2e` wraps the legacy flag.

## 5. Verification

- [x] 5.1 `anchor build` from clean checkout: green — produces `target/deploy/enclz.so`, `target/deploy/enclz-keypair.json`, `target/idl/enclz.json`
- [x] 5.2 `cargo test --package enclz`: all 14 unit tests pass
- [x] 5.3 `anchor test` (against local validator): green via `npm run test:e2e` (= `anchor test --validator legacy`); 2 mocha tests pass
- [x] 5.4 `git status` after build: no `target/` or `.anchor/` artifacts tracked
