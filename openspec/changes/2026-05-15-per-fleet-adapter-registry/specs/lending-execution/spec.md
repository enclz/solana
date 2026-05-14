## REMOVED Requirements

### Requirement: execute_lending_op instruction signature and account constraints

The `execute_lending_op` instruction is removed from the core program. Lending functionality moves to first-party adapter programs — `enclz-kamino-adapter` for Kamino markets and `enclz-save-adapter` for Save markets (each deployed under a separate program ID, Apache-2.0, out-of-tree). Callers invoke lending functionality via `execute_via_adapter` against the appropriate adapter's program ID, which must be registered in the calling group's `GroupAdapterRegistry`.

**Reason for removal:** core program minimization (per-fleet adapter registry change). Lending functionality is preserved with the same semantic contract — deposit and withdraw against whitelisted lending markets, with policy enforcement on the input amount — but now lives in separately-deployed programs governed by the same on-chain registry the owner controls. Removing the instruction from the core shrinks the audit surface and unblocks long-term immutability of the core.

### Requirement: execute_lending_op cpi_data pass-through

The opaque `cpi_data` pattern (where the backend passes pre-encoded CPI bytes through the core) is replaced with the adapter pattern. The adapter is now responsible for building the protocol's CPI from structured inputs, validating those inputs against the adapter's stored `constraints` (e.g., allowed market PDAs), and signing the CPI with the agent_wallet PDA seeds. This restores type safety at the cost of one adapter program per lending protocol family.

### Requirement: execute_lending_op whitelist binding

The owner-approved-adapter check in `execute_via_adapter` replaces the whitelist binding on lending venues. Now the owner must explicitly register the lending adapter (e.g., `enclz-kamino-adapter`) via `add_adapter`, optionally with `constraints: Vec<u8>` encoding the allowed market list. If the owner has not registered the adapter — or if the adapter rejects the call against its `constraints` — lending calls fail at the appropriate level (`AdapterNotApproved` from the core or a typed adapter error).

### Requirement: execute_lending_op deposit and withdraw paths

Both deposit and withdraw paths are preserved in the adapter programs. The deposit/withdraw distinction becomes an argument inside `call_data` parsed by the adapter, rather than a discriminator on the core instruction.
