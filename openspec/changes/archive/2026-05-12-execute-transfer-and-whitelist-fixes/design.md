## Context

The program has six open issues spanning execute_transfer behavior and whitelist management. None have been addressed. Issues #33 (additive fee), #31 (init_if_needed), and #30 (RecipientInvalid) all modify the same instruction and must be implemented together. Issues #29 (whitelist target field) and #11 (InvalidEntryType) are independent schema/error changes. Issue #18 is a spec-only correction.

The current execute_transfer uses a subtractive fee, requires the recipient ATA to pre-exist, and has no guard against transfers to protocol-reserved accounts. The WhitelistEntry account stores no target address, making it impossible for the SPA to render entries without off-chain tracking. The error enum uses `Unauthorized` for invalid entry types, which is semantically wrong.

## Goals / Non-Goals

**Goals:**
- Make protocol fee additive so recipients receive exactly the requested amount
- Auto-create recipient ATA via `init_if_needed`, eliminating the backend's pre-instruction
- Reject transfers to protocol fee wallet or agent PDA with a typed `RecipientInvalid` error before Anchor's duplicate-mut check fires
- Store `target` on WhitelistEntry so the SPA can render addresses from on-chain data alone
- Replace misleading `Unauthorized` with `InvalidEntryType` for unknown whitelist entry types
- Fix the OpenSpec spec's claim that missing PDAs return `WhitelistViolation` (they return `AccountNotInitialized`)

**Non-Goals:**
- No changes to execute_swap or execute_lending_op (they have their own fee patterns and account structures)
- No migration ix for the WhitelistEntry schema — devnet wipe is acceptable
- No changes to the backend error mapping (that's a separate repo); this pass only adds the new error variants to the enum

## Decisions

1. **Additive fee with ceil rounding.** `fee = ceil(amount * 10 / 10000)` = `(amount * 10 + 9999) / 10000`. Ceil rounds in favor of the protocol (minimum 1 unit fee per transfer). This is a behavioral change for micro-amounts — previously, amounts < 1000 had zero fee; now every positive amount incurs at least 1 unit.

2. **Constraint-level RecipientInvalid check.** Issue #30 proposed handler-level `require_keys_neq!`, but `ConstraintDuplicateMutableAccount` fires during Anchor's account resolution, before the handler runs. Instead, add `constraint` attributes on the new `recipient_wallet: UncheckedAccount` account, which Anchor evaluates during per-account constraint checking (before the cross-account duplicate-mut check). This produces a typed `RecipientInvalid` (6014) instead of opaque 2040.

3. **New `recipient_wallet` account enables both #30 and #31.** Adding `recipient_wallet: UncheckedAccount` serves double duty: it's the authority for ATA `init_if_needed` and the subject of the RecipientInvalid constraints. The whitelist PDA seed changes from `to_token_account.owner.as_ref()` to `recipient_wallet.key().as_ref()` because the ATA may not exist yet (the owner field is only set after initialization, so can't be read at resolution time).

4. **Append new error variants at the end of the enum.** Adding `RecipientInvalid` and `InvalidEntryType` after the existing 14 variants preserves the existing Anchor error codes (6000-6013). The backend's name→code mapping only changes for new variants; existing codes stay stable.

5. **Devnet wipe for schema change.** The WhitelistEntry struct grows from 98 to 130 bytes (adding `target: Pubkey`). Existing accounts won't deserialize. Devnet wipe is the cleanest option — `remove_from_whitelist` all entries, redeploy, re-add.

6. **spent_today counter uses request `amount`, not `total`.** The daily/per-tx limits represent the value the orchestrator intends the agent to send out. The fee is an overhead paid by the agent, not a spend. Counting `total` would mean the fee consumes from the agent's spend capacity, which is wrong — the agent should be able to send `amount` worth of value regardless of the fee overhead.

## Risks / Trade-offs

- [Devnet wipe] All existing WhitelistEntry accounts become invalid after redeploy → Mitigation: orchestrators re-add their entries; acceptable for devnet
- [Backend coupling] Backend must update error maps to handle `RecipientInvalid` (6014) and `InvalidEntryType` (6015) → Mitigation: backend already handles 6000-band errors generically; adding two new codes is additive
- [Micro-amount behavior] Ceil rounding means transfers of 1-999 units now incur a 1-unit fee (previously zero) → Acceptable: USDC-decimals (1 unit = 0.000001 USDC) make this negligible for human-scale transactions
- [Whitelist seed change] Existing execute_transfer callers must supply `recipient_wallet` and `mint` accounts → Mitigation: the backend constructs these accounts already for other instructions; this adds the same accounts to execute_transfer
