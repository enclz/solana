# Enclz

Onchain spend-policy enforcement for AI agent fleets on Solana. Orchestrators define agent identities and recipient allowlists onchain; a backend operator submits transfers that the program validates against per-agent spend limits, hourly caps, and whitelist entries before executing the SPL token transfer.

See `docs/SPECIFICATION.md` for the full architectural reference.

## Major Concepts

### State (PDAs)

| Account | Seeds | Purpose |
|---|---|---|
| `GroupConfig` | `["group", owner]` | Orchestrator policy admin record |
| `AgentWallet` | `["wallet", group, agent_index]` | Per-agent vault with daily / per-tx / hourly limits, replay nonce |
| `WhitelistEntry` | `["whitelist", group, target_address]` | Recipient allowlist entry with optional TTL and approved amount |

Default limits: `daily=10 USDC`, `per_tx=1 USDC`, `hourly_cap=5`. Protocol fee: `10 bps`.

### Errors

`EnclzError` variants map 1:1 to backend REST error codes: `WhitelistViolation`, `WhitelistExpired`, `WhitelistAmountExhausted`, `DailyLimitExceeded`, `PerTxLimitExceeded`, `HourlyCapExceeded`, `NonceMismatch`, `Unauthorized`, `InvalidAmount`, `InvalidTtl`. Names are stable — backend pass-through depends on them.

### Backend / direct program integration

External code that calls the program directly (the Enclz backend, composing programs, or auditing tools) can consume typed bindings via `@enclz/sdk`:

```typescript
import { IDL, PROGRAM_ID, type Enclz } from "@enclz/sdk";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

const program = new Program<Enclz>(IDL, AnchorProvider.env());
```

See the [Distribution](#distribution) section for build and publish instructions. AI agents using the Agent REST API do not need this package.

## Setup

### Prerequisites

- Rust (Cargo) — 1.85+
- Solana CLI (Agave) — install via `sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"`
- Anchor CLI 1.0.1 — install via `cargo install --git https://github.com/solana-foundation/anchor avm --locked` then `avm install 1.0.1 && avm use 1.0.1`
- Node.js 20+ (npm)

### Install

```bash
npm install
```

### Environment

```bash
cp .env.example .env
# fill in QUICKNODE_DEVNET_RPC_URL with your devnet RPC endpoint
```

`.env` is gitignored. Commands that need these vars must run through `dotenv-cli`:

```bash
npx dotenv -- anchor deploy --provider.cluster devnet
# or via the convenience script
npm run deploy:devnet
```

### Deployer keypairs

Project-local keypairs live under `.solana/keys/` (gitignored). Generate one for devnet and fund it:

```bash
solana-keygen new --no-bip39-passphrase --silent --outfile .solana/keys/devnet-deployer.json
chmod 600 .solana/keys/devnet-deployer.json
solana-keygen pubkey .solana/keys/devnet-deployer.json
# paste the printed pubkey into https://faucet.solana.com/ to fund
```

`Anchor.toml`'s `[provider.devnet]` already points at this path.

## Testing

```bash
cargo test --package enclz   # Rust unit tests — PDA derivation, INIT_SPACE, errors, constants
npm run test:e2e             # mocha integration tests against solana-test-validator (wraps `anchor test --validator legacy` because Anchor 1.0 defaults to surfpool)
```

## Deploy

```bash
anchor build
npm run deploy:devnet
```

`npm run deploy:devnet` wraps `migrations/deploy.ts` and:

- detects program-ID drift between `target/deploy/enclz-keypair.json`,
  `declare_id!` in `programs/enclz/src/lib.rs`, and `Anchor.toml`, patching the
  source files and rebuilding before deploying;
- skips a redundant deploy if `target/deploy/enclz.so` already matches the
  bytes deployed at `<program-id>` on the chosen cluster.

Mainnet deploys go through `npm run deploy:mainnet -- --force-mainnet`. The
`--force-mainnet` flag is intentionally awkward: only pass it once the
upgrade authority has been transferred to a Squads multisig.

After a successful devnet deploy, run the smoke suite end-to-end:

```bash
npm run smoke:devnet
```

It exercises the full happy path (provision → fund → 5 × $1 transfer →
auto-void → reject 6th → reject stale-nonce) against the live cluster.

## Security & quality gates

- [`SECURITY.md`](SECURITY.md) — disclosure policy and reporting contact.
- `solana_security_txt!` is embedded in the program; auditors can fetch it via
  `query-security-txt <PROGRAM_ID>`.
- `.github/workflows/program-ci.yml` runs `anchor build`, `cargo test`,
  `anchor test --validator legacy`, `cargo tarpaulin` (gated at 85% overall /
  90% on `execute_transfer.rs`), `cargo audit`, and `cargo deny check` on every PR.

## Distribution

### `@enclz/sdk` npm package

The `sdk/` directory contains the `@enclz/sdk` package, which re-exports the Anchor IDL JSON, the `Enclz` TypeScript type, and `PROGRAM_ID` for direct on-chain callers (program composability, auditing tools, custom backends). AI agents should use the Agent REST API + MCP server instead — see `sdk/README.md` for positioning details.

```bash
npm run build:sdk     # anchor build (if needed) + copy artifacts + tsc → sdk/dist/
npm run publish:sdk   # build:sdk + cd sdk && npm publish --access public
```

The package version is single-sourced from `programs/enclz/Cargo.toml` via IDL `metadata.version` and written to `sdk/package.json` automatically during `npm run build:sdk`.

### On-chain IDL

After deploying to a cluster for the first time, publish the IDL on-chain so tooling and explorers can resolve it without installing `@enclz/sdk`:

```bash
npm run idl:init:devnet      # first deploy on devnet
npm run idl:upgrade:devnet   # every subsequent upgrade on devnet
npm run idl:init:mainnet     # first deploy on mainnet
npm run idl:upgrade:mainnet  # every subsequent upgrade on mainnet
```

These scripts require the deployer keypair to be the IDL upgrade authority. Env vars (`QUICKNODE_DEVNET_RPC_URL`, `MAINNET_RPC_URL`) must be set via `.env` — the scripts run through `dotenv-cli` matching the `deploy:*` pattern.

## Project layout

```
programs/enclz/         Anchor program crate
├── src/lib.rs          declare_id, module wiring, unit tests
├── src/constants.rs    seed prefixes, default limits, protocol fee
├── src/errors.rs       EnclzError enum
└── src/state/          GroupConfig, AgentWallet, WhitelistEntry

tests/                  TypeScript integration tests (mocha)
tests/smoke.ts          end-to-end smoke against a live cluster (run via npm run smoke:devnet)
migrations/deploy.ts    devnet/mainnet deploy entrypoint with program-ID drift + idempotent re-deploy
scripts/                CI helpers: check-coverage
.github/workflows/      program-ci.yml — build, test, coverage, audit gates
.solana/keys/           deployer keypairs (gitignored)
openspec/               OpenSpec change proposals and capability specs
docs/                   product + architectural specification (submodule)
```
