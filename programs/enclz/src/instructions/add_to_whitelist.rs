use anchor_lang::prelude::*;

use crate::constants::WHITELIST_SEED;
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type::{EXTERNAL, INTRA_GROUP, PROTOCOL};
use crate::state::{GroupConfig, WhitelistEntry};

#[derive(Accounts)]
#[instruction(target_address: Pubkey)]
pub struct AddToWhitelistAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        init,
        payer = owner,
        space = WhitelistEntry::DISCRIMINATOR.len() + WhitelistEntry::INIT_SPACE,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), target_address.as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

pub fn handle_add_to_whitelist(
    context: Context<AddToWhitelistAccountConstraints>,
    _target_address: Pubkey,
    label: [u8; 32],
    entry_type: u8,
    ttl_expires_at: i64,
    approved_amount: u64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let (stored_ttl, stored_amount) = match entry_type {
        INTRA_GROUP => return err!(EnclzError::Unauthorized),
        EXTERNAL => {
            require!(ttl_expires_at > now, EnclzError::InvalidTtl);
            require!(approved_amount > 0, EnclzError::InvalidAmount);
            (ttl_expires_at, approved_amount)
        }
        PROTOCOL => (0, 0),
        _ => return err!(EnclzError::Unauthorized),
    };

    let whitelist_entry = &mut context.accounts.whitelist_entry;
    whitelist_entry.label = label;
    whitelist_entry.added_by = context.accounts.owner.key();
    whitelist_entry.entry_type = entry_type;
    whitelist_entry.ttl_expires_at = stored_ttl;
    whitelist_entry.approved_amount = stored_amount;
    whitelist_entry.amount_used = 0;
    whitelist_entry.bump = context.bumps.whitelist_entry;
    Ok(())
}
