# group-provisioning Specification

## Purpose
TBD - created by archiving change add-owner-instructions. Update Purpose after archive.
## Requirements
### Requirement: initialize_group instruction

The program SHALL expose `initialize_group(group_name: [u8; 32], backend_operator: Pubkey, protocol_fee_wallet: Pubkey, dex_router: Pubkey)` that creates a `GroupConfig` PDA owned by the signer with `group_name` written verbatim into the account, and atomically creates a type-2 `WhitelistEntry` for `dex_router`. The handler SHALL NOT validate the encoding of `group_name`.

#### Scenario: Successful group creation
- **WHEN** signer calls `initialize_group` with a 32-byte name and valid pubkeys
- **THEN** a `GroupConfig` PDA exists with `owner == signer`, `backend_operator` + `protocol_fee_wallet` set, `agent_count == 0`, `group_name` byte-equal to the input, and a type-2 `WhitelistEntry` PDA exists for `dex_router`

#### Scenario: DEX router auto-whitelisted at init
- **WHEN** signer calls `initialize_group` with `dex_router = JUPITER_PROGRAM_ID`
- **THEN** a `WhitelistEntry` PDA with seeds `["whitelist", group, JUPITER_PROGRAM_ID]` and `entry_type == 2` is created atomically in the same transaction

#### Scenario: Duplicate group rejected
- **WHEN** the same signer calls `initialize_group` twice
- **THEN** the second call fails because the PDA already exists

#### Scenario: Non-UTF-8 name accepted
- **WHEN** signer calls `initialize_group` with a `group_name` containing arbitrary bytes (e.g. `[0xFF; 32]`)
- **THEN** the handler succeeds and stores the bytes verbatim — no `InvalidArgument` or other validation error is raised

### Requirement: add_agent instruction

The program SHALL expose `add_agent(display_name: [u8;32], daily_limit: Option<u64>, per_tx_limit: Option<u64>, hourly_tx_cap: Option<u8>)` callable only by the group owner. It creates an `AgentWallet` PDA, captures the supplied `mint` account's pubkey into `AgentWallet.mint` (binding the agent to that single SPL mint for its lifetime), creates the agent's ATA for that mint via CPI, auto-creates an intra-group `WhitelistEntry` (entry_type=0, ttl=0, amount=0) for the new agent's pubkey, and increments `GroupConfig.agent_count`.

#### Scenario: Successful add with defaults
- **WHEN** owner calls `add_agent` with all `Option` args `None` and a `mint` account
- **THEN** the agent's `daily_limit == 10_000_000`, `per_tx_limit == 1_000_000`, `hourly_tx_cap == 5`, `AgentWallet.mint == mint.key()`, the ATA is created against that mint, the intra-group whitelist entry exists, and `agent_count` increments by 1

#### Scenario: Override applied
- **WHEN** owner calls `add_agent` with `daily_limit: Some(50_000_000)`
- **THEN** stored `daily_limit == 50_000_000`; other limits use defaults; mint is still captured from the supplied `mint` account

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer calls `add_agent`
- **THEN** the call fails with `Unauthorized` before handler executes

#### Scenario: Mint binding is permanent
- **WHEN** an agent has been provisioned with mint A and the owner later wishes to operate the same agent against mint B
- **THEN** there is no instruction that mutates `AgentWallet.mint`; the owner must add a new agent via a second `add_agent` call

### Requirement: update_agent_limits instruction

The program SHALL expose `update_agent_limits(daily_limit: Option<u64>, per_tx_limit: Option<u64>, hourly_tx_cap: Option<u8>)` callable only by the group owner; `Some` values overwrite the field, `None` leaves it unchanged.

#### Scenario: Patch a single field
- **WHEN** owner calls with `daily_limit: Some(5_000_000)`, others `None`
- **THEN** only `daily_limit` changes; `per_tx_limit` and `hourly_tx_cap` retain prior values

### Requirement: update_backend_operator instruction

The program SHALL expose `update_backend_operator(new_operator: Pubkey)` callable only by the group owner; replaces `GroupConfig.backend_operator`.

#### Scenario: Operator rotation
- **WHEN** owner calls `update_backend_operator` with a new pubkey
- **THEN** subsequent `execute_transfer` calls signed by the old operator fail and ones signed by the new operator succeed

### Requirement: emergency_withdraw instruction

The program SHALL expose `emergency_withdraw(agent_index: u8)` callable only by the group owner. The destination ATA is supplied as an account in the instruction's account list. The agent ATA SHALL be constrained to `token::authority = agent_wallet`; the destination ATA SHALL be constrained to `destination_token_account.mint == agent_token_account.mint` (mint *parity* — owner can sweep any mint the agent has accumulated via swaps, but both legs of a single call must agree to prevent typo-driven cross-mint transfers). No standalone `Mint` account is required in the `Accounts` struct — mint identity is read from the supplied token accounts. The handler bypasses spend limits and operator nonce, transferring the full agent ATA balance via SPL token CPI to the destination ATA.

#### Scenario: Sweep bound-mint funds
- **WHEN** owner calls `emergency_withdraw` against an agent ATA of mint == `agent_wallet.mint` holding any positive balance, with a destination ATA of the same mint
- **THEN** all tokens are transferred to the destination ATA and the agent ATA balance is zero

#### Scenario: Sweep non-bound-mint funds accumulated via swaps
- **WHEN** owner calls `emergency_withdraw` against an agent ATA of any mint M (not necessarily `agent_wallet.mint`) that the agent acquired via prior swaps, with a destination ATA also of mint M
- **THEN** all tokens are transferred to the destination ATA and the agent ATA balance is zero — the absence of an absolute mint pin is what makes non-bound-mint sweeps possible

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer (including the backend operator) calls `emergency_withdraw`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Mint mismatch between agent and destination ATAs rejected
- **WHEN** owner passes an `agent_token_account` of mint A and a `destination_token_account` of mint B (A ≠ B)
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint` before any transfer

#### Scenario: Per-mint sweep
- **WHEN** an agent holds positive balances of three different mints (e.g., the bound mint plus two swap residuals) and owner wants to recover all of them
- **THEN** owner invokes `emergency_withdraw` three times, once per mint, each with its own pair of agent + destination ATAs of that mint

