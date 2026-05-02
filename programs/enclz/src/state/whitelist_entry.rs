use anchor_lang::prelude::*;

pub mod entry_type {
    pub const INTRA_GROUP: u8 = 0;
    pub const EXTERNAL: u8 = 1;
    pub const PROTOCOL: u8 = 2;
}

#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    pub label: [u8; 32],
    pub added_by: Pubkey,
    pub entry_type: u8,
    pub ttl_expires_at: i64,
    pub approved_amount: u64,
    pub amount_used: u64,
    pub bump: u8,
}
