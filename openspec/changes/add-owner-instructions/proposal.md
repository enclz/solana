## Why

After the Anchor workspace + state structs land, the orchestrator needs on-chain instructions to provision groups, mint agent wallets, and manage the whitelist. These are all owner-signed admin ops — none of them move user funds, so we ship them ahead of `execute_transfer` to unblock backend integration testing of the provisioning flow.

## What Changes

- New owner-signed instructions in `programs/enclz/`:
  - `initialize_group` — creates `GroupConfig` PDA, records `backend_operator` + `protocol_fee_wallet`.
  - `add_agent` — creates `AgentWallet` PDA + USDC ATA via CPI to `associated_token::create`. Auto-creates intra-group `WhitelistEntry` (entry_type=0). Increments `agent_count`. Applies template defaults if `Option` args are `None`.
  - `add_to_whitelist` — creates `WhitelistEntry` PDA. Validates: type 1 requires `ttl > now` and `approved_amount > 0`; type 0/2 force `ttl=0` and `amount=0`.
  - `renew_whitelist_entry` — type-1 only; rejects `ttl <= now` or `approved_amount < amount_used`.
  - `remove_from_whitelist` — closes PDA, returns rent; rejects type 0.
  - `update_agent_limits` — patches `daily_limit` / `per_tx_limit` / `hourly_tx_cap` via `Option` args.
  - `update_backend_operator` — rotates the operator pubkey on `GroupConfig`.
  - `emergency_withdraw` — bypasses limits, sweeps full agent ATA balance to a destination address.
- Every instruction enforces `signer == GroupConfig.owner` (except `initialize_group` where signer becomes owner).
- Per-instruction unit + integration tests covering happy path + every reject branch.

## Capabilities

### New Capabilities
- `group-provisioning`: `initialize_group`, `add_agent`, `update_backend_operator`, `update_agent_limits`, `emergency_withdraw`
- `whitelist-management`: `add_to_whitelist`, `renew_whitelist_entry`, `remove_from_whitelist`

### Modified Capabilities
<!-- none — purely additive -->

## Impact

- Adds `programs/enclz/src/instructions/{initialize_group,add_agent,add_to_whitelist,renew_whitelist_entry,remove_from_whitelist,update_agent_limits,update_backend_operator,emergency_withdraw}.rs`.
- Wires entry points in `lib.rs`.
- New tests under `tests/` and `programs/enclz/tests/`.
- Backend can now call provisioning endpoints end-to-end on devnet (after `add-devnet-deploy-pipeline` ships).
- Depends on `init-anchor-workspace` being merged first.
