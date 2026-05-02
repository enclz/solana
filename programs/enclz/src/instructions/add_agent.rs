use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{
    DEFAULT_DAILY_LIMIT, DEFAULT_HOURLY_CAP, DEFAULT_PER_TX_LIMIT, WALLET_SEED, WHITELIST_SEED,
};
use crate::state::whitelist_entry::entry_type;
use crate::state::{AgentWallet, GroupConfig, WhitelistEntry};

#[derive(Accounts)]
pub struct AddAgentAccountConstraints<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        init,
        payer = owner,
        space = AgentWallet::DISCRIMINATOR.len() + AgentWallet::INIT_SPACE,
        seeds = [WALLET_SEED, group_config.key().as_ref(), &[group_config.agent_count]],
        bump,
    )]
    pub agent_wallet: Account<'info, AgentWallet>,

    #[account(
        init,
        payer = owner,
        space = WhitelistEntry::DISCRIMINATOR.len() + WhitelistEntry::INIT_SPACE,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), agent_wallet.key().as_ref()],
        bump,
    )]
    pub intra_group_entry: Account<'info, WhitelistEntry>,

    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = agent_wallet,
    )]
    pub agent_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

pub fn handle_add_agent(
    context: Context<AddAgentAccountConstraints>,
    display_name: [u8; 32],
    daily_limit: Option<u64>,
    per_tx_limit: Option<u64>,
    hourly_tx_cap: Option<u8>,
) -> Result<()> {
    let agent_wallet = &mut context.accounts.agent_wallet;
    agent_wallet.group = context.accounts.group_config.key();
    agent_wallet.display_name = display_name;
    agent_wallet.daily_limit = daily_limit.unwrap_or(DEFAULT_DAILY_LIMIT);
    agent_wallet.per_tx_limit = per_tx_limit.unwrap_or(DEFAULT_PER_TX_LIMIT);
    agent_wallet.hourly_tx_cap = hourly_tx_cap.unwrap_or(DEFAULT_HOURLY_CAP);
    agent_wallet.spent_today = 0;
    agent_wallet.tx_count_this_hour = 0;
    agent_wallet.last_spend_reset = 0;
    agent_wallet.last_hour_reset = 0;
    agent_wallet.operator_nonce = 0;
    agent_wallet.bump = context.bumps.agent_wallet;

    let intra_group_entry = &mut context.accounts.intra_group_entry;
    intra_group_entry.label = display_name;
    intra_group_entry.added_by = context.accounts.owner.key();
    intra_group_entry.entry_type = entry_type::INTRA_GROUP;
    intra_group_entry.ttl_expires_at = 0;
    intra_group_entry.approved_amount = 0;
    intra_group_entry.amount_used = 0;
    intra_group_entry.bump = context.bumps.intra_group_entry;

    let group_config = &mut context.accounts.group_config;
    group_config.agent_count = group_config
        .agent_count
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}
