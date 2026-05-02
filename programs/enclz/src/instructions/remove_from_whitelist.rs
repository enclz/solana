use anchor_lang::prelude::*;

use crate::constants::WHITELIST_SEED;
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{GroupConfig, WhitelistEntry};

#[derive(Accounts)]
#[instruction(target_address: Pubkey)]
pub struct RemoveFromWhitelistAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        mut,
        close = owner,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), target_address.as_ref()],
        bump = whitelist_entry.bump,
        constraint = whitelist_entry.entry_type != entry_type::INTRA_GROUP @ EnclzError::Unauthorized,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}

pub fn handle_remove_from_whitelist(
    _context: Context<RemoveFromWhitelistAccountConstraints>,
    _target_address: Pubkey,
) -> Result<()> {
    Ok(())
}
