## 1. Deploy script

- [ ] 1.1 Create `migrations/deploy.ts` invoking `anchor deploy --provider.cluster devnet`
- [ ] 1.2 Parse program ID from deploy output; if changed, patch `declare_id!` in `lib.rs` and `[programs.devnet]` in `Anchor.toml`
- [ ] 1.3 Add npm script `"deploy:devnet": "ts-node migrations/deploy.ts"`
- [ ] 1.4 Document required env: `QUICKNODE_DEVNET_URL`, `ANCHOR_WALLET`

## 2. Smoke test

- [ ] 2.1 Create `tests/smoke.ts` that:
  - generates a fresh keypair, airdrops SOL
  - pre-creates `protocol_fee_wallet` USDC ATA via `getOrCreateAssociatedTokenAccount`
  - calls `initialize_group(backend_operator, protocol_fee_wallet, JUPITER_PROGRAM_ID)` — creates DEX router type-2 whitelist entry atomically
  - calls `add_agent` with `hourly_tx_cap: Some(10)` to avoid `HourlyCapExceeded` before whitelist exhaustion
  - calls `add_to_whitelist` (external, $5 cap, ttl=now+3600)
  - funds agent ATA via swap or test-mint
  - executes 5 × $1 `execute_transfer`, asserts whitelist PDA closed after 5th
  - attempts a 6th transfer, asserts `WhitelistViolation`
  - submits a stale-nonce transfer, asserts `NonceMismatch`
- [ ] 2.2 Add npm script `"smoke:devnet": "ts-node tests/smoke.ts"`
- [ ] 2.3 Run smoke against devnet end-to-end; fix any RPC / ATA / CPI issues surfaced

## 3. CI workflow

- [ ] 3.1 Create `.github/workflows/program-ci.yml`
- [ ] 3.2 Job `build`: checkout + install solana + anchor + rust toolchain + run `anchor build`
- [ ] 3.3 Job `test`: `cargo test --package enclz` then `anchor test --skip-build`
- [ ] 3.4 Job `coverage`: install `cargo-tarpaulin`, run `cargo tarpaulin --packages enclz --out Xml`, parse, fail if instruction coverage < 85% on any of `execute_transfer.rs`, `execute_swap.rs`, `execute_lending_op.rs`; `execute_transfer.rs` threshold is 90%
- [ ] 3.5 Job `audit`: `cargo install cargo-audit cargo-deny`, run both, fail on critical
- [ ] 3.6 Workflow triggers: push to `main`, PR to `main`

## 4. Security.txt + checklist

- [ ] 4.1 Add `solana-security-txt = "1"` dep, embed `solana_security_txt!` macro in `lib.rs` with name "Enclz", project_url, contacts, source_code, audit (placeholder)
- [ ] 4.2 Create `docs/SECURITY_REVIEW.md` with per-instruction checklist (signer, ownership, arithmetic, seeds, ATA mint), one row per instruction, sign-off slots
- [ ] 4.3 Walk every instruction handler; tick each item or fix the gap

## 5. Dependency policy

- [ ] 5.1 Create `deny.toml` denying GPL/AGPL, warning on duplicate versions
- [ ] 5.2 Run `cargo deny check`; resolve any current findings

## 5.5 Upgrade authority

- [ ] 5.5.1 Create `docs/UPGRADE_AUTHORITY.md` documenting devnet authority (deploy keypair) vs. mainnet authority (Squads multisig required before user funds)
- [ ] 5.5.2 Add `deploy.ts` guard: if `--provider.cluster mainnet-beta` and `ANCHOR_WALLET` is a single-sig key, print warning and require `--force-mainnet` flag to proceed

## 5.6 Error map publication

- [ ] 5.6.1 After first successful devnet deploy, generate `idl/error-map.json` by extracting Anchor IDL errors array and mapping each `{ code, name }` to `{ anchorCode, restErrorCode: camelCase(name) }`
- [ ] 5.6.2 Commit `idl/error-map.json` alongside `idl/enclz.json`
- [ ] 5.6.3 Add CI step: diff `target/idl/enclz.json` errors vs `idl/error-map.json` codes; fail if drift

## 6. IDL publication

- [ ] 6.1 After first successful devnet deploy, copy `target/idl/enclz.json` → `idl/enclz.json`
- [ ] 6.2 Commit `idl/enclz.json` with the resolved program ID
- [ ] 6.3 Add CI step that diffs `target/idl/enclz.json` against `idl/enclz.json` and fails if drift exists without commit

## 7. Verification

- [ ] 7.1 `anchor run deploy:devnet`: deploys cleanly from fresh checkout
- [ ] 7.2 `anchor run smoke:devnet`: exits 0
- [ ] 7.3 CI workflow green on a probe PR
- [ ] 7.4 `query-security-txt <program-id>` returns embedded fields
- [ ] 7.5 SECURITY_REVIEW.md fully signed off
- [ ] 7.6 Backend team confirms they can build against `idl/enclz.json` + deployed program ID
- [ ] 7.7 Backend team confirms `/v1/swap`, `/v1/deposit`, `/v1/withdraw` routes integrate correctly against the deployed program on devnet
