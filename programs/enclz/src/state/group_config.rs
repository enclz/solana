use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct GroupConfig {
    pub owner: Pubkey,
    pub backend_operator: Pubkey,
    pub protocol_fee_wallet: Pubkey,
    pub agent_count: u8,
    pub group_name: [u8; 32],
}
