use anchor_lang::prelude::*;

use crate::constants::WHITELIST_SEED;
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{GroupConfig, WhitelistEntry};

#[derive(Accounts)]
#[instruction(target_address: Pubkey)]
pub struct RenewWhitelistEntryAccountConstraints<'info> {
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        mut,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), target_address.as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}

pub fn handle_renew_whitelist_entry(
    context: Context<RenewWhitelistEntryAccountConstraints>,
    _target_address: Pubkey,
    ttl_expires_at: i64,
) -> Result<()> {
    let entry = &mut context.accounts.whitelist_entry;
    require!(
        entry.entry_type == entry_type::EXTERNAL,
        EnclzError::Unauthorized
    );

    let now = Clock::get()?.unix_timestamp;
    require!(ttl_expires_at > now, EnclzError::InvalidTtl);

    entry.ttl_expires_at = ttl_expires_at;
    Ok(())
}
