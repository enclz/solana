use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::WALLET_SEED;
use crate::errors::EnclzError;
use crate::state::{AgentWallet, GroupConfig};

#[derive(Accounts)]
#[instruction(agent_index: u8)]
pub struct EmergencyWithdrawAccountConstraints<'info> {
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        constraint = agent_wallet.group == group_config.key() @ EnclzError::Unauthorized,
        seeds = [WALLET_SEED, group_config.key().as_ref(), &[agent_index]],
        bump = agent_wallet.bump,
    )]
    pub agent_wallet: Account<'info, AgentWallet>,

    #[account(
        mut,
        token::authority = agent_wallet,
    )]
    pub agent_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = destination_token_account.mint == agent_token_account.mint @ EnclzError::InvalidMint,
    )]
    pub destination_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_emergency_withdraw(
    context: Context<EmergencyWithdrawAccountConstraints>,
    agent_index: u8,
) -> Result<()> {
    let amount = context.accounts.agent_token_account.amount;
    if amount == 0 {
        return Ok(());
    }

    let group_key = context.accounts.group_config.key();
    let agent_bump = context.accounts.agent_wallet.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        WALLET_SEED,
        group_key.as_ref(),
        &[agent_index],
        &[agent_bump],
    ]];

    let cpi_accounts = Transfer {
        from: context.accounts.agent_token_account.to_account_info(),
        to: context.accounts.destination_token_account.to_account_info(),
        authority: context.accounts.agent_wallet.to_account_info(),
    };
    let cpi_context = CpiContext::new_with_signer(
        context.accounts.token_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_context, amount)?;
    Ok(())
}
