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
