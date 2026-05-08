## MODIFIED Requirements

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
