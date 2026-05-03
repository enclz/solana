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
- [x] 2.2 Add npm script `"smoke:devnet": "dotenv -- ts-node --transpile-only tests/smoke.ts"` (`--transpile-only` matches `ts-mocha`'s default and sidesteps Anchor 0.30.1's strict `ResolvedAccounts` typings on `.accounts({...})`, which the existing `*.spec.ts` files also rely on)
- [x] 2.3 Smoke test green against devnet end-to-end: `initialize_group` → `add_agent` → `add_to_whitelist` → 5×`execute_transfer` (whitelist auto-closes) → 6th transfer rejected (`AccountNotInitialized` 3012) → second-merchant flow with stale-nonce replay → rejected with `NonceMismatch`. Run from `4ES2AnX6dYD5rq3o3nE8CeV58nFKSMKS6yNf5vC5XbVM` against the QuickNode devnet RPC

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
- [x] 5.2 `cargo deny check` passes locally: `advisories ok, bans ok, licenses ok, sources ok`. Three `unmatched-source` warnings (allowed sources `solana-foundation/anchor`, `anza-xyz`, `solana-labs` not currently encountered by any crate) are informational and reflect that all current deps come through crates.io rather than git — the allowlist is in place for when the project starts pinning git revs

## 5.5 Upgrade authority

- [x] 5.5.1 Add `deploy.ts` guard: if `--mainnet` and `--force-mainnet` is not set, refuse the deploy. Intent: friction-only safeguard against single-sig fat-fingering before the upgrade authority is transferred to a Squads multisig.

## 6. Verification

- [x] 6.1 `npm run deploy:devnet`: deploys cleanly from fresh checkout. Initial deploy of `67i3uY4gZaidynKa8XbNW569qACSVCebwKnLpNYVtWjj` to devnet succeeded (upgrade authority `4ES2AnX6dYD5rq3o3nE8CeV58nFKSMKS6yNf5vC5XbVM`, ProgramData `BV246ifprsvqSWbwMBEqNA4oq9TQBhcTmVnph4ze7uU7`)
- [x] 6.2 `npm run smoke:devnet`: exits 0 — see 2.3
- [ ] 6.3 CI workflow green on a probe PR (pending — workflow runs after first push)
- [x] 6.4 Embedded `security.txt` section verified on-chain at `67i3uY4gZaidynKa8XbNW569qACSVCebwKnLpNYVtWjj` after upgrading to the freshly-built `.so`. Fields: name=Enclz, project_url=https://github.com/enclz/solana, contacts=email:security@enclz.dev, policy=https://github.com/enclz/solana/blob/main/SECURITY.md, preferred_languages=en, source_code=https://github.com/enclz/solana, source_release=v0.1.0, auditors=None. `query-security-txt` 1.1.2 itself fails to compile against the current Solana toolchain (`agave-feature-set` E0308) — verified instead via `solana program dump` + parsing the `BEGIN SECURITY.TXT V1` / `END SECURITY.TXT V1` markers. Also surfaced a deploy-script gap: the idempotence check hashes local vs deployed binaries but does not ensure the local `.so` is rebuilt from current sources, so a stale `target/deploy/enclz.so` (predating the `security_txt!` macro) silently deployed on the first run; the redeploy after `rm target/deploy/enclz.so && anchor build` shipped the corrected binary
- [ ] 6.5 Backend team confirms they can integrate against the deployed program ID (deferred — external coordination)
