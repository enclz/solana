use anchor_lang::prelude::*;

#[constant]
pub const GROUP_SEED: &[u8] = b"group";

#[constant]
pub const WALLET_SEED: &[u8] = b"wallet";

#[constant]
pub const WHITELIST_SEED: &[u8] = b"whitelist";

pub const DEFAULT_DAILY_LIMIT: u64 = 10_000_000;
pub const DEFAULT_PER_TX_LIMIT: u64 = 1_000_000;
pub const DEFAULT_HOURLY_CAP: u8 = 5;

pub const PROTOCOL_FEE_BPS: u16 = 10;

// Jupiter Aggregator v6 program. Pinned for documentation + CI smoke tests;
// runtime CPI authorization is enforced via a type-2 (PROTOCOL) WhitelistEntry
// keyed on the program ID, so the orchestrator can rotate to a new aggregator
// version by updating the whitelist without redeploying this program.
#[constant]
pub const JUPITER_V6_PROGRAM_ID: Pubkey =
    pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
