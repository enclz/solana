# transfer-execution Specification Delta

## MODIFIED Requirements

### Requirement: execute_transfer instruction signature and account constraints

The program SHALL expose `execute_transfer(amount: u64, expected_nonce: u64, agent_index: u8)` callable only by the `backend_operator` recorded on the agent's `GroupConfig`. The `agent_index` parameter reconstructs the `agent_wallet` PDA seed for the SPL token CPI signer; instruction args remain (`amount`, `expected_nonce`) plus this implementation-required index.

Required accounts: `backend_operator` (signer), `group_config`, `group_owner` (writable, address-bound to `group_config.owner` — receives auto-void rent), `agent_wallet` (writable), `from_token_account` (writable, owner == agent_wallet PDA, mint == `agent_wallet.mint`), `recipient_wallet` (unchecked, pubkey constrained != protocol_fee_wallet and != agent_wallet PDA), `to_token_account` (writable, init_if_needed via ATA with `recipient_wallet` as authority, mint == `agent_wallet.mint`), `whitelist_entry` (seeds derived from `recipient_wallet.key()`), `protocol_fee_token_account` (writable, owner == `group_config.protocol_fee_wallet`, mint == `agent_wallet.mint`), `mint` (account matching `agent_wallet.mint`), `token_program`, `associated_token_program`, `system_program`.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_transfer`
- **THEN** the call fails with `Unauthorized`

#### Scenario: from_token_account ownership enforced
- **WHEN** caller passes a `from_token_account` whose `owner` is not the `agent_wallet` PDA
- **THEN** Anchor account constraint rejects the transaction before handler executes

#### Scenario: from_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `from_token_account` whose `mint` differs from `agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint` before any state mutation

#### Scenario: to_token_account auto-created if missing
- **WHEN** the recipient does not yet have an ATA for the agent's mint
- **THEN** the `to_token_account` is initialized via `init_if_needed` with `backend_operator` as payer; the transfer proceeds normally

#### Scenario: protocol_fee_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `protocol_fee_token_account` whose `mint != agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint`

#### Scenario: protocol_fee_token_account misroute rejected
- **WHEN** caller passes a `protocol_fee_token_account` whose `owner` is not `group_config.protocol_fee_wallet`
- **THEN** Anchor account constraint rejects the transaction

#### Scenario: Recipient wallet equals protocol fee wallet
- **WHEN** `recipient_wallet.key()` equals `group_config.protocol_fee_wallet`
- **THEN** Anchor constraint rejects the transaction with `RecipientInvalid` before the duplicate-mut check fires

#### Scenario: Recipient wallet equals agent PDA
- **WHEN** `recipient_wallet.key()` equals `agent_wallet.key()`
- **THEN** Anchor constraint rejects the transaction with `RecipientInvalid`

#### Scenario: Whitelist seed bound to recipient_wallet
- **WHEN** caller supplies a valid `whitelist_entry` PDA but `recipient_wallet.key()` does not match the PDA's seed target
- **THEN** Anchor seed constraint rejects the transaction — no whitelist bypass possible via account substitution

### Requirement: Whitelist enforcement

The instruction SHALL require that the supplied `whitelist_entry` PDA matches seeds `["whitelist", group_config, recipient_wallet.key()]` and exists. The seed is derived from `recipient_wallet.key()` (not from `to_token_account.owner`, since the ATA may be uninitialized at resolution time) — so it is impossible to pair a valid whitelist PDA with an unwhitelisted destination. For `entry_type == 1` it SHALL additionally enforce TTL and amount-cap.

#### Scenario: Recipient not whitelisted
- **WHEN** no `WhitelistEntry` PDA exists for the recipient address
- **THEN** Anchor's typed account constraint rejects the transaction with `AccountNotInitialized` (3012) during account resolution, before the handler runs; the backend translates this to `whitelist_violation` for the REST response

#### Scenario: External entry expired
- **WHEN** `entry_type == 1` and `now > ttl_expires_at`
- **THEN** the call fails with `WhitelistExpired`

#### Scenario: External entry amount exhausted
- **WHEN** `entry_type == 1` and `amount_used + amount > approved_amount`
- **THEN** the call fails with `WhitelistAmountExhausted`

#### Scenario: Intra-group transfer always allowed within spend limits
- **WHEN** `entry_type == 0` and all spend-limit checks pass
- **THEN** the transfer succeeds regardless of TTL or amount fields

#### Scenario: Protocol entry always allowed within spend limits
- **WHEN** `entry_type == 2` and all spend-limit checks pass
- **THEN** the transfer succeeds regardless of TTL or amount fields

### Requirement: Protocol fee deduction

The instruction SHALL compute `protocol_fee = ceil(amount * 10 / 10_000)` using integer ceil arithmetic (`(amount * 10 + 9999) / 10_000`), compute `total = amount + protocol_fee`, transfer `amount` to the recipient ATA, and transfer `protocol_fee` to the `protocol_fee_token_account`. Both transfers happen via `token::transfer` CPI signed by the agent wallet PDA. The total drained from the agent's `from_token_account` is `total` (= `amount + protocol_fee`).

#### Scenario: Fee math with standard amount
- **WHEN** `amount = 1_000_000` (1 USDC)
- **THEN** `protocol_fee == 1_000` and `total == 1_001_000`; recipient receives exactly `1_000_000`, fee wallet receives `1_000`

#### Scenario: Fee math with small amount
- **WHEN** `amount = 99`
- **THEN** `protocol_fee == 1` (ceil) and `total == 100`; recipient receives exactly `99`, fee wallet receives `1`

#### Scenario: Fee math with zero amount
- **WHEN** `amount = 0`
- **THEN** the handler rejects with `InvalidAmount` before reaching fee computation

#### Scenario: Fee transfer failure aborts whole instruction
- **WHEN** the fee leg fails (e.g., fee ATA missing)
- **THEN** the entire transaction reverts and the amount leg is rolled back

### Requirement: Counter and consumption updates after successful transfer

The instruction SHALL increment `spent_today` by the gross `amount` (not `total`, and not `amount - fee`) and `tx_count_this_hour` by 1. For `entry_type == 1` it SHALL also increment `whitelist_entry.amount_used` by `amount`.

#### Scenario: Spent_today counts request amount
- **WHEN** a transfer of `amount = 1_000_000` succeeds
- **THEN** `spent_today` increases by `1_000_000`, not by `total` (which is `1_001_000`)

#### Scenario: Hourly counter increments
- **WHEN** a transfer succeeds
- **THEN** `tx_count_this_hour` increases by exactly 1

#### Scenario: Amount used incremented for external entry
- **WHEN** a transfer to an `entry_type == 1` recipient succeeds
- **THEN** `whitelist_entry.amount_used` increases by `amount`

## ADDED Requirements

### Requirement: recipient_wallet constraint enforcement

The program SHALL constrain the `recipient_wallet` account such that its pubkey is not equal to `group_config.protocol_fee_wallet` and not equal to `agent_wallet.key()`. These constraints SHALL be evaluated by Anchor during per-account resolution, before the cross-account duplicate-mutable-account check.

#### Scenario: Recipient is protocol fee wallet
- **WHEN** `recipient_wallet` equals `group_config.protocol_fee_wallet`
- **THEN** Anchor constraint evaluation rejects with `RecipientInvalid` before any account-level duplicate-mut check

#### Scenario: Recipient is agent PDA
- **WHEN** `recipient_wallet` equals `agent_wallet.key()`
- **THEN** Anchor constraint evaluation rejects with `RecipientInvalid`

### Requirement: init_if_needed for recipient ATA

The program SHALL auto-create the recipient's associated token account when it does not already exist, using Anchor's `init_if_needed` with `associated_token::mint = mint` and `associated_token::authority = recipient_wallet`. The `backend_operator` SHALL pay rent for any newly created ATA.

#### Scenario: Existing ATA reused
- **WHEN** the recipient already has an ATA for the mint
- **THEN** `init_if_needed` is a no-op and the transfer proceeds normally

#### Scenario: New ATA created
- **WHEN** the recipient does not yet have an ATA for the mint
- **THEN** Anchor initializes the ATA at the canonical address, `backend_operator` pays the rent, and the transfer proceeds
