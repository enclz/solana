use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AgentWallet {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub display_name: [u8; 32],
    pub daily_limit: u64,
    pub per_tx_limit: u64,
    pub hourly_tx_cap: u8,
    pub spent_today: u64,
    pub tx_count_this_hour: u8,
    pub last_spend_reset: i64,
    pub last_hour_reset: i64,
    pub operator_nonce: u64,
    pub bump: u8,
}
