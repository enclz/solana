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
materialize_key SOLANA_DEVNET_DEPLOYER_KEYPAIR  .solana/keys/devnet-deployer.json
materialize_key SOLANA_TESTNET_DEPLOYER_KEYPAIR .solana/keys/testnet-deployer.json
materialize_key SOLANA_MAINNET_DEPLOYER_KEYPAIR .solana/keys/mainnet-deployer.json
