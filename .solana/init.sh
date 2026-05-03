#!/bin/bash
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

materialize_key() {
  local var_name="$1"
  local out_path="$2"
  local value="${!var_name:-}"
  if [ -z "$value" ]; then
    echo "$var_name not set — skipping $out_path" >&2
    return 0
  fi
  mkdir -p "$(dirname "$out_path")"
  umask 077
  printf '%s' "$value" > "$out_path"
  chmod 600 "$out_path"
  echo "Materialized $var_name → $out_path"
}

cd "$CLAUDE_PROJECT_DIR"

# Project deps — cloud sessions start without node_modules. Tests, deploy
# scripts, and dotenv-cli all require them. Re-runs only when package.json
# is newer than node_modules so steady-state session starts stay fast.
if [ ! -d node_modules ] || [ package.json -nt node_modules ]; then
  npm install
fi

materialize_key SOLANA_DEVNET_DEPLOYER_KEYPAIR  .solana/keys/devnet-deployer.json
materialize_key SOLANA_TESTNET_DEPLOYER_KEYPAIR .solana/keys/testnet-deployer.json
materialize_key SOLANA_MAINNET_DEPLOYER_KEYPAIR .solana/keys/mainnet-deployer.json

# Program-ID keypair: pubkey must match declare_id! in lib.rs and Anchor.toml.
# Without it, anchor build generates a fresh random ID and e2e tests can't deploy.
materialize_key SOLANA_PROGRAM_ID_KEYPAIR       target/deploy/enclz-keypair.json
