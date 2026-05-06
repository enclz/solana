## MODIFIED Requirements

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
