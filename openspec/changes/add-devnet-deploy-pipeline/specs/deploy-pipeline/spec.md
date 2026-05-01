## ADDED Requirements

### Requirement: Devnet deploy script

The repo SHALL include `migrations/deploy.ts` that runs `anchor deploy --provider.cluster devnet`, prints the resulting program ID, and updates `declare_id!` + `Anchor.toml` if the ID changed.

#### Scenario: Fresh deploy
- **WHEN** developer runs `anchor run deploy:devnet` on a clean machine with `QUICKNODE_DEVNET_URL` set
- **THEN** the program is deployed, the program ID is printed, and source files are patched if needed

#### Scenario: Idempotent re-deploy
- **WHEN** developer runs the deploy script a second time without source changes
- **THEN** the script reports "no upgrade needed" and exits 0

### Requirement: End-to-end smoke test

The repo SHALL include `tests/smoke.ts` that, against devnet, performs:
1. Pre-create `protocol_fee_wallet` USDC ATA via `getOrCreateAssociatedTokenAccount`
2. Call `initialize_group(backend_operator, protocol_fee_wallet, JUPITER_PROGRAM_ID)` — also creates DEX router type-2 whitelist entry
3. Call `add_agent` with `hourly_tx_cap: Some(10)` (override default of 5 so the 6th transfer hits `WhitelistViolation`, not `HourlyCapExceeded`)
4. Call `add_to_whitelist` for an external merchant ($5 cap, TTL = now + 3600)
5. Fund agent ATA via airdrop + swap (or test-mint on devnet)
6. Execute 5 × $1 `execute_transfer` to merchant — assert `amount_used` increments each time
7. Assert `WhitelistEntry` PDA is closed after 5th transfer
8. Attempt 6th transfer to same merchant — assert `WhitelistViolation`
9. Submit transfer with stale nonce — assert `NonceMismatch`

#### Scenario: All steps pass
- **WHEN** smoke test runs against a freshly deployed program on devnet
- **THEN** the script exits 0 and prints transaction signatures for every step

#### Scenario: Any step fails
- **WHEN** any expected outcome differs from spec
- **THEN** the script exits non-zero with a clear error pointing to the failed step

### Requirement: IDL artifact committed

After the first successful deploy, `target/idl/enclz.json` SHALL be committed to the repo at a stable path (`idl/enclz.json` mirroring `target/idl/enclz.json`) so the backend can consume it without running `anchor build`.

#### Scenario: IDL exists in repo
- **WHEN** backend developer clones the repo
- **THEN** `idl/enclz.json` is present and validates against the deployed program ID

#### Scenario: IDL drift caught in PR
- **WHEN** any change modifies the program's public surface (instruction args, account structs)
- **THEN** the diff to `idl/enclz.json` appears in the same PR
