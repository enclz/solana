use anchor_lang::prelude::*;

use crate::constants::{GROUP_SEED, WHITELIST_SEED};
use crate::state::whitelist_entry::entry_type;
use crate::state::{GroupConfig, WhitelistEntry};

#[derive(Accounts)]
#[instruction(group_name: [u8; 32], backend_operator: Pubkey, protocol_fee_wallet: Pubkey, dex_router: Pubkey)]
pub struct InitializeGroupAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = GroupConfig::DISCRIMINATOR.len() + GroupConfig::INIT_SPACE,
        seeds = [GROUP_SEED, owner.key().as_ref()],
        bump,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        init,
        payer = owner,
        space = WhitelistEntry::DISCRIMINATOR.len() + WhitelistEntry::INIT_SPACE,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), dex_router.as_ref()],
        bump,
    )]
    pub dex_router_entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_group(
    context: Context<InitializeGroupAccountConstraints>,
    group_name: [u8; 32],
    backend_operator: Pubkey,
    protocol_fee_wallet: Pubkey,
    _dex_router: Pubkey,
) -> Result<()> {
    let group_config = &mut context.accounts.group_config;
    group_config.owner = context.accounts.owner.key();
    group_config.backend_operator = backend_operator;
    group_config.protocol_fee_wallet = protocol_fee_wallet;
    group_config.agent_count = 0;
    group_config.group_name = group_name;

    let dex_router_entry = &mut context.accounts.dex_router_entry;
    dex_router_entry.label = *b"dex-router\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
    dex_router_entry.added_by = context.accounts.owner.key();
    dex_router_entry.entry_type = entry_type::PROTOCOL;
    dex_router_entry.ttl_expires_at = 0;
    dex_router_entry.approved_amount = 0;
    dex_router_entry.amount_used = 0;
    dex_router_entry.bump = context.bumps.dex_router_entry;
    Ok(())
}
