## MODIFIED Requirements

### Requirement: AgentWallet PDA

The program SHALL define an `AgentWallet` account derived from seeds `["wallet", group_pubkey, agent_index]` containing the group pubkey, the SPL token mint the agent is bound to (set once at `add_agent` and immutable thereafter), a fixed 32-byte display name, daily/per-tx/hourly limits in token-native 6-decimal units, current-period counters, last-reset timestamps, a `u64 operator_nonce`, and a `bump: u8` storing the canonical PDA bump.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives `[b"wallet", group.as_ref(), &[idx]]` for any group + index
- **THEN** result matches the program-side PDA bump

#### Scenario: Default limits applied at init
- **WHEN** an `AgentWallet` is initialized with `daily_limit: None`, `per_tx_limit: None`, `hourly_tx_cap: None`
- **THEN** values default to 10_000_000, 1_000_000, and 5 respectively

#### Scenario: Mint round-trips unchanged
- **WHEN** test serializes an `AgentWallet` populated with an arbitrary `mint: Pubkey` and deserializes the buffer
- **THEN** the decoded `mint` is byte-for-byte equal to the input

#### Scenario: INIT_SPACE accommodates all fields including mint
- **WHEN** test reads `AgentWallet::INIT_SPACE`
- **THEN** the value equals `32 (group) + 32 (mint) + 32 (display_name) + 8 + 8 + 1 + 8 + 1 + 8 + 8 + 8 + 1 (bump) = 149`
