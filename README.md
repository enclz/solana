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

## Cloud sessions (Claude Code on the web)

`.claude/settings.json` registers a `SessionStart` hook that runs `.solana/init.sh`. The hook materializes deployer keypairs from environment variables into `.solana/keys/<cluster>-deployer.json` — but only when `CLAUDE_CODE_REMOTE=true`, so locally it's a no-op.

In the cloud environment configuration, set:

- `QUICKNODE_DEVNET_RPC_URL` — same as local
- `SOLANA_DEVNET_DEPLOYER_KEYPAIR` — JSON byte array (no quotes), e.g. `[12,34,...,99]`. Add `SOLANA_TESTNET_DEPLOYER_KEYPAIR` / `SOLANA_MAINNET_DEPLOYER_KEYPAIR` analogously if/when needed.
- `SOLANA_PROGRAM_ID_KEYPAIR` — JSON byte array of the local `target/deploy/enclz-keypair.json`. Required for `npm run test:e2e` in the cloud, since `anchor test` deploys the program at this keypair's pubkey and it must match `declare_id!` in `lib.rs`. Without it the hook skips silently and only `cargo test` works.

And in the setup script, install Solana CLI + Anchor:

```bash
#!/bin/bash
set -euo pipefail
ANCHOR_VERSION="1.0.1"
SOLANA_CHANNEL="stable"

if ! command -v solana >/dev/null 2>&1; then
  sh -c "$(curl -sSfL "https://release.anza.xyz/${SOLANA_CHANNEL}/install")"
fi
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"

if ! command -v avm >/dev/null 2>&1; then
  cargo install --git https://github.com/solana-foundation/anchor avm --force --locked
fi
avm install "$ANCHOR_VERSION"
avm use "$ANCHOR_VERSION"

cat > /etc/profile.d/solana-anchor.sh <<'EOF'
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
EOF
```

Network access must be set to **Custom** with `release.anza.xyz` added (the rest is in the Trusted defaults).

## Project layout

```
programs/enclz/         Anchor program crate
├── src/lib.rs          declare_id, module wiring, unit tests
├── src/constants.rs    seed prefixes, default limits, protocol fee
├── src/errors.rs       EnclzError enum
└── src/state/          GroupConfig, AgentWallet, WhitelistEntry

tests/                  TypeScript integration tests (mocha)
.solana/keys/           deployer keypairs (gitignored)
.solana/init.sh         cloud-session keypair materialization hook
openspec/               OpenSpec change proposals and capability specs
docs/                   product + architectural specification
```
