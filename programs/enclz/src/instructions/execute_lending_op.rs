use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{WALLET_SEED, WHITELIST_SEED};
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{AgentWallet, GroupConfig, WhitelistEntry};
use crate::util::cpi::invoke_protocol_cpi;
use crate::util::fee::compute_fee;
use crate::util::time::{needs_daily_reset, needs_hourly_reset};

pub mod op_type {
    pub const DEPOSIT: u8 = 0;
    pub const WITHDRAW: u8 = 1;
}

// `agent_index` reconstructs the agent_wallet PDA seed for the lending CPI
// signer (same convention as `execute_transfer`). `cpi_data` is the raw
// lending-program instruction payload prepared by the backend; the program
// is authorized via a type-2 (PROTOCOL) WhitelistEntry keyed on its address.
#[derive(Accounts)]
#[instruction(op_type: u8, amount: u64, expected_nonce: u64, agent_index: u8)]
pub struct ExecuteLendingOpAccountConstraints<'info> {
    pub backend_operator: Signer<'info>,

    #[account(
        has_one = backend_operator @ EnclzError::Unauthorized,
    )]
    pub group_config: Box<Account<'info, GroupConfig>>,

    #[account(
        mut,
        seeds = [WALLET_SEED, group_config.key().as_ref(), &[agent_index]],
        bump = agent_wallet.bump,
    )]
    pub agent_wallet: Box<Account<'info, AgentWallet>>,

    #[account(
        mut,
        constraint = agent_token_account.owner == agent_wallet.key() @ EnclzError::InvalidTokenAccount,
        constraint = agent_token_account.mint == agent_wallet.mint @ EnclzError::InvalidMint,
    )]
    pub agent_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), lending_program.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Box<Account<'info, WhitelistEntry>>,

    #[account(
        mut,
        constraint = protocol_fee_token_account.owner == group_config.protocol_fee_wallet @ EnclzError::InvalidFeeAccount,
        constraint = protocol_fee_token_account.mint == agent_wallet.mint @ EnclzError::InvalidMint,
    )]
    pub protocol_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: program is authorized via the type-2 whitelist_entry keyed on
    /// `lending_program.key()`; the seed binding plus entry_type assertion
    /// makes a non-whitelisted program unreachable.
    pub lending_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_execute_lending_op<'info>(
    context: Context<'info, ExecuteLendingOpAccountConstraints<'info>>,
    op_type_arg: u8,
    amount: u64,
    expected_nonce: u64,
    agent_index: u8,
    cpi_data: Vec<u8>,
) -> Result<()> {
    // Reject unknown discriminants up front — keeps the rest of the handler
    // dealing only with `DEPOSIT` or `WITHDRAW`.
    require!(
        op_type_arg == op_type::DEPOSIT || op_type_arg == op_type::WITHDRAW,
        EnclzError::InvalidAmount
    );
    require!(amount > 0, EnclzError::InvalidAmount);

    // Step 1: nonce check before any other state read.
    let agent_wallet = &mut context.accounts.agent_wallet;
    require!(
        expected_nonce == agent_wallet.operator_nonce,
        EnclzError::NonceMismatch
    );

    // Step 2: bump nonce.
    agent_wallet.operator_nonce = agent_wallet
        .operator_nonce
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    // Step 3: roll counters across day/hour boundaries.
    let now = Clock::get()?.unix_timestamp;
    if needs_daily_reset(agent_wallet.last_spend_reset, now) {
        agent_wallet.spent_today = 0;
        agent_wallet.last_spend_reset = now;
    }
    if needs_hourly_reset(agent_wallet.last_hour_reset, now) {
        agent_wallet.tx_count_this_hour = 0;
        agent_wallet.last_hour_reset = now;
    }

    // Steps 4–6: limits applied to gross `amount`.
    require!(
        amount <= agent_wallet.per_tx_limit,
        EnclzError::PerTxLimitExceeded
    );
    let projected_spent = agent_wallet
        .spent_today
        .checked_add(amount)
        .ok_or(EnclzError::InvalidAmount)?;
    require!(
        projected_spent <= agent_wallet.daily_limit,
        EnclzError::DailyLimitExceeded
    );
    require!(
        agent_wallet.tx_count_this_hour < agent_wallet.hourly_tx_cap,
        EnclzError::HourlyCapExceeded
    );

    // Step 7: only type-2 (PROTOCOL) whitelist entries authorize a lending CPI.
    require!(
        context.accounts.whitelist_entry.entry_type == entry_type::PROTOCOL,
        EnclzError::WhitelistViolation
    );

    // Signer seeds reused by every CPI in this instruction.
    let group_key = context.accounts.group_config.key();
    let agent_bump = agent_wallet.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        WALLET_SEED,
        group_key.as_ref(),
        &[agent_index],
        &[agent_bump],
    ]];

    // Step 8: dispatch on op_type.
    if op_type_arg == op_type::DEPOSIT {
        // Deposit: fee out of principal, lending receives `net_principal`.
        let (_net_principal, fee) = compute_fee(amount)?;

        if fee > 0 {
            let cpi_accounts = Transfer {
                from: context.accounts.agent_token_account.to_account_info(),
                to: context.accounts.protocol_fee_token_account.to_account_info(),
                authority: agent_wallet.to_account_info(),
            };
            let cpi_context = CpiContext::new_with_signer(
                context.accounts.token_program.key(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_context, fee)?;
        }

        invoke_protocol_cpi(
            &context.accounts.lending_program,
            context.remaining_accounts,
            cpi_data,
            signer_seeds,
        )?;
    } else {
        // Withdraw: snapshot agent ATA balance, redeem via lending CPI, take
        // fee out of the delta. Net (`redeemed - fee`) lands in the agent ATA.
        let pre_balance = context.accounts.agent_token_account.amount;

        invoke_protocol_cpi(
            &context.accounts.lending_program,
            context.remaining_accounts,
            cpi_data,
            signer_seeds,
        )?;

        // Reload the ATA so we observe the redeemed delta credited by the CPI.
        context.accounts.agent_token_account.reload()?;
        let post_balance = context.accounts.agent_token_account.amount;
        let redeemed = post_balance
            .checked_sub(pre_balance)
            .ok_or(EnclzError::InvalidAmount)?;
        require!(redeemed > 0, EnclzError::InvalidAmount);

        let (_net_redeemed, fee) = compute_fee(redeemed)?;
        if fee > 0 {
            let cpi_accounts = Transfer {
                from: context.accounts.agent_token_account.to_account_info(),
                to: context.accounts.protocol_fee_token_account.to_account_info(),
                authority: agent_wallet.to_account_info(),
            };
            let cpi_context = CpiContext::new_with_signer(
                context.accounts.token_program.key(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_context, fee)?;
        }
    }

    // Step 9: counters reflect gross `amount` (input to the instruction).
    agent_wallet.spent_today = projected_spent;
    agent_wallet.tx_count_this_hour = agent_wallet
        .tx_count_this_hour
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    Ok(())
}
