## Why

After all instructions are implemented + unit-tested, we need a repeatable path to put the program on devnet so the backend team can integrate against a real RPC. We also need a hardening pass — coverage gates, dependency audit, security.txt — before the program can be considered audit-ready. Bundling these into one change keeps deployment artifacts and quality gates together.

## What Changes

- `migrations/deploy.ts` script that runs `anchor deploy --provider.cluster devnet`, captures the program ID, and patches `declare_id!` + `Anchor.toml`.
- `tests/smoke.ts` end-to-end script that exercises the full happy path against devnet: init group → add agent → add whitelist → fund agent ATA → execute transfer → verify on Solana Explorer.
- CI workflow (`.github/workflows/program-ci.yml`) that on every push:
  - Runs `anchor build`.
  - Runs `cargo test`.
  - Runs `anchor test` against local validator.
  - Runs `cargo tarpaulin` and fails if instruction-code coverage < 85% (or < 90% on `execute_transfer.rs`).
  - Runs `cargo audit` and `cargo deny check`.
- `solana-security-txt` macro added to the program metadata (contact email, source URL, audit status placeholder).
- Devnet program ID + IDL JSON published to `target/idl/enclz.json` and committed for backend consumption.

## Capabilities

### New Capabilities
- `deploy-pipeline`: deployment script, smoke test, IDL publication
- `program-hardening`: CI quality gates, security.txt, dependency policy

### Modified Capabilities
<!-- none — additive -->

## Impact

- Adds `migrations/deploy.ts`, `tests/smoke.ts`, `.github/workflows/program-ci.yml`, `deny.toml`.
- Modifies `programs/enclz/src/lib.rs` to embed `solana_security_txt!`.
- Commits `target/idl/enclz.json` after first devnet deploy.
- Backend can now build against the real program ID and IDL.
- Depends on `init-anchor-workspace`, `add-owner-instructions`, `add-execute-transfer` all being merged.
- Mainnet deploy is explicitly **out of scope** — handled by the `/deploy-to-mainnet` skill after external audit.
