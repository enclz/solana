## 1. Deploy script

- [x] 1.1 Create `migrations/deploy.ts` invoking `anchor deploy --provider.cluster devnet`
- [x] 1.2 Parse program ID from deploy output; if changed, patch `declare_id!` in `lib.rs` and `[programs.devnet]` in `Anchor.toml`
- [x] 1.3 Add npm script `"deploy:devnet": "ts-node migrations/deploy.ts"` (wrapped through `dotenv -- ts-node migrations/deploy.ts --devnet` so `.env` still loads)
- [x] 1.4 Document required env: `QUICKNODE_DEVNET_RPC_URL`, `ANCHOR_WALLET` (header docstring in `migrations/deploy.ts`)

## 2. Smoke test

- [x] 2.1 Create `tests/smoke.ts` that:
  - generates a fresh keypair, funds it from the deployer fee payer (devnet airdrop quotas are fragile)
  - pre-creates `protocol_fee_wallet` USDC ATA via `getOrCreateAssociatedTokenAccount`
  - calls `initialize_group(backend_operator, protocol_fee_wallet, dex_router)` — creates DEX router type-2 whitelist entry atomically
  - calls `add_agent` with `hourly_tx_cap: Some(10)` to avoid `HourlyCapExceeded` before whitelist exhaustion
  - calls `add_to_whitelist` (external, $5 cap, ttl=now+3600)
  - funds agent ATA via fresh test mint (devnet USDC mint authority is unreachable; same approach as Anchor integration tests)
  - executes 5 × $1 `execute_transfer`, asserts whitelist PDA closed after 5th
  - attempts a 6th transfer, asserts the call reverts (whitelist PDA missing)
  - submits a stale-nonce transfer against a second merchant, asserts `NonceMismatch`
- [x] 2.2 Add npm script `"smoke:devnet": "dotenv -- ts-node tests/smoke.ts"`
- [ ] 2.3 Run smoke against devnet end-to-end; fix any RPC / ATA / CPI issues surfaced (deferred — requires a devnet deploy + funded `ANCHOR_WALLET`)

## 3. CI workflow

- [x] 3.1 Create `.github/workflows/program-ci.yml`
- [x] 3.2 Job `build`: checkout + install solana + anchor + rust toolchain + run `anchor build`
- [x] 3.3 Job `test`: `cargo test --package enclz` then `anchor test --validator legacy` (project uses solana-test-validator per `design.md`)
- [x] 3.4 Job `coverage`: install `cargo-tarpaulin`, run `cargo tarpaulin --packages enclz --out Xml`, parse with `scripts/check-coverage.mjs`. Threshold is configured (85% / 90%) but the **gate is informational** (`continue-on-error: true`): tarpaulin instruments the host build, while `programs/enclz/tests/` execute the BPF binary via litesvm, so instruction-handler lines look uncovered even though the 26-test suite exercises them end-to-end. Promoting this to a hard gate requires SBF-aware coverage tooling (cargo-llvm-cov with sbpf, or the Solana fork of grcov) — out of scope for this change
- [x] 3.5 Job `audit`: `cargo install cargo-audit cargo-deny`, run both, fail on critical
- [x] 3.6 Workflow triggers: push to `main`, PR to `main`

## 4. Security.txt

- [x] 4.1 Add `solana-security-txt = "1.1"` dep, embed `solana_security_txt!` macro in `lib.rs` with name "Enclz", project_url, contacts, source_code, audit (placeholder); `policy` field points at `SECURITY.md`

## 5. Dependency policy

- [x] 5.1 Create `deny.toml` denying GPL/AGPL, warning on duplicate versions
- [ ] 5.2 Run `cargo deny check`; resolve any current findings (deferred — requires `cargo install cargo-deny` in dev env)

## 5.5 Upgrade authority

- [x] 5.5.1 Add `deploy.ts` guard: if `--mainnet` and `--force-mainnet` is not set, refuse the deploy. Intent: friction-only safeguard against single-sig fat-fingering before the upgrade authority is transferred to a Squads multisig.

## 5.6 Error map publication

- [x] 5.6.1 Generate `idl/error-map.json` by extracting Anchor IDL errors array and mapping each `{ code, name }` to `{ anchorCode, name, restErrorCode: camelCase(name) }` (script `scripts/sync-idl.mjs`)
- [x] 5.6.2 Commit `idl/error-map.json` alongside `idl/enclz.json` (initial map seeded from `EnclzError`; will be re-emitted from real IDL after first deploy)
- [x] 5.6.3 Add CI step: diff `target/idl/enclz.json` errors vs `idl/error-map.json` codes; fail if drift (`scripts/check-idl-sync.mjs` + `idl-drift` job)

## 6. IDL publication

- [ ] 6.1 After first successful devnet deploy, copy `target/idl/enclz.json` → `idl/enclz.json` (script `scripts/sync-idl.mjs`; awaiting first deploy)
- [ ] 6.2 Commit `idl/enclz.json` with the resolved program ID (awaiting first deploy)
- [x] 6.3 Add CI step that diffs `target/idl/enclz.json` against `idl/enclz.json` and fails if drift exists without commit (`scripts/check-idl-sync.mjs` + `idl-drift` job)

## 7. Verification

- [ ] 7.1 `npm run deploy:devnet`: deploys cleanly from fresh checkout (deferred — requires devnet RPC + funded deployer)
- [ ] 7.2 `npm run smoke:devnet`: exits 0 (deferred — depends on 7.1)
- [ ] 7.3 CI workflow green on a probe PR (deferred — verified after first PR runs the workflow)
- [ ] 7.4 `query-security-txt <program-id>` returns embedded fields (deferred — depends on 7.1)
- [ ] 7.5 Backend team confirms they can build against `idl/enclz.json` + deployed program ID (deferred — depends on 7.1)
