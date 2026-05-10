## Why

Six open GitHub issues (#33, #31, #30, #29, #18, #11) describe bugs and enhancements in the Enclz program's execute_transfer and whitelist subsystems that are blocking production use cases. The protocol fee is subtractive (breaks x402 payment flows), the recipient ATA must pre-exist (adding backend complexity and an extra instruction), transfers to protocol-reserved accounts surface opaque Anchor errors, whitelist entries don't store their target address (breaks SPA listing), error messages for invalid entry types are misleading, and one spec scenario contradicts actual Anchor behavior.

None of these have been implemented — the code, `docs/SPECIFICATION.md`, and the OpenSpec specs all agree on the current (broken) state.

## What Changes

- **Make protocol fee additive** (issue #33): `execute_transfer` computes `total = amount + ceil(amount * 10 / 10000)` instead of `net = amount - fee`. Recipient receives exactly `amount`; fee wallet receives `fee`. **BREAKING** — fee math changes; backend must update fee expectations.
- **Auto-create recipient ATA** (issue #31): `to_token_account` uses `init_if_needed` with ATA constraints, eliminating the backend's `createAssociatedTokenAccountIdempotentInstruction` pre-instruction. Adds `recipient_wallet`, `mint`, and `associated_token_program` accounts to the struct.
- **Reject transfers to reserved accounts with typed error** (issue #30): New `RecipientInvalid` error fires when `recipient_wallet == protocol_fee_wallet` or `recipient_wallet == agent_wallet PDA`, preventing the opaque `ConstraintDuplicateMutableAccount` (2040).
- **Store whitelist target on WhitelistEntry** (issue #29): Adds `target: Pubkey` field so the SPA can render addresses without `knownTargets` workaround. **BREAKING** — schema change; devnet wipe required.
- **Semantic error for invalid entry types** (issue #11): New `InvalidEntryType` replaces misleading `Unauthorized` in `add_to_whitelist` for unknown entry types.
- **Fix spec drift** (issue #18): `openspec/specs/transfer-execution/spec.md` updated to match actual Anchor behavior (missing whitelist PDA → `AccountNotInitialized`, not `WhitelistViolation`).

## Capabilities

### New Capabilities

*(none — all changes modify existing capabilities)*

### Modified Capabilities

- `transfer-execution`: protocol fee changes from subtractive to additive; recipient ATA changes from "must exist" to auto-created via `init_if_needed`; new `RecipientInvalid` enforcement step; whitelist PDA seed derivation changes from `to_token_account.owner` to `recipient_wallet.key()`; missing-PDA behavior corrected from `WhitelistViolation` to `AccountNotInitialized`
- `whitelist-management`: `add_to_whitelist` returns `InvalidEntryType` (not `Unauthorized`) for unknown entry types; `WhitelistEntry` stores its `target` address
- `program-state`: `WhitelistEntry` account gains a `target: Pubkey` field

## Impact

- **Program** (`programs/enclz/src/`): error enum (2 new variants), `WhitelistEntry` struct (+1 field), `execute_transfer` accounts (4 new/modified), `add_to_whitelist` error assignment, `add_agent` field writes, `compute_fee` rewrite, error offset test update
- **Docs**: `docs/SPECIFICATION.md` and `openspec/specs/transfer-execution/spec.md` updated
- **SDK**: Regenerated from IDL after `anchor build`
- **Deploy**: Devnet wipe required for WhitelistEntry schema migration
- **Backend**: Must map `RecipientInvalid` (6014) and `InvalidEntryType` (6015); must drop `createAssociatedTokenAccount` pre-instruction; must update fee math expectations
