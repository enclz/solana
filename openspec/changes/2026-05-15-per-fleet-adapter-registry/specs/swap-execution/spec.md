## REMOVED Requirements

### Requirement: execute_swap instruction signature and account constraints

The `execute_swap` instruction is removed from the core program. Swap functionality moves to a first-party adapter program (`enclz-jupiter-adapter`, deployed under a separate program ID, Apache-2.0, out-of-tree). Callers invoke swap functionality via `execute_via_adapter` against the Jupiter adapter's program ID, which must be registered in the calling group's `GroupAdapterRegistry`.

**Reason for removal:** core program minimization (per-fleet adapter registry change). Swap functionality is preserved with the same semantic contract — Jupiter-v6 quote-driven swap of two SPL tokens, with policy enforcement on the input amount — but now lives in a separately-deployed program governed by the same on-chain registry the owner controls. Removing the instruction from the core shrinks the audit surface and unblocks long-term immutability of the core.

### Requirement: execute_swap protocol fee deduction

Subsumed by `execute_via_adapter`'s policy enforcement plus the adapter's internal fee handling. Removed from the core's responsibility surface.

### Requirement: execute_swap whitelist binding

The owner-approved-adapter check in `execute_via_adapter` replaces the implicit "swap is always allowed if the agent has a swap-capable mint" semantics. Now the owner must explicitly register the Jupiter adapter via `add_adapter` before the agent can execute swaps. If the owner has not registered the adapter, swap calls fail with `AdapterNotApproved`.

### Requirement: execute_swap account constraints

All swap-specific account validation (input/output mints, ATA ownership, Jupiter route data, etc.) moves to the adapter program. The core's `execute_via_adapter` only validates that writable token accounts owned by the agent_wallet PDA remain so after the CPI (custody post-check).
