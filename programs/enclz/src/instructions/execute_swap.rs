use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{WALLET_SEED, WHITELIST_SEED};
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{AgentWallet, GroupConfig, WhitelistEntry};
use crate::util::fee::compute_fee;
use crate::util::time::{needs_daily_reset, needs_hourly_reset};

// `agent_index` reconstructs the agent_wallet PDA seed for the SPL transfer
// CPI signer (same convention as `execute_transfer`). `route_data` is the raw
// Jupiter v6 instruction payload; the backend constructs it with
// `net_amount_in = amount_in - protocol_fee` and the desired slippage so
// Jupiter's own `minimum_amount_out` check applies to the post-fee net.
#[derive(Accounts)]
#[instruction(amount_in: u64, minimum_amount_out: u64, expected_nonce: u64, agent_index: u8)]
pub struct ExecuteSwapAccountConstraints<'info> {
    pub backend_operator: Signer<'info>,

    #[account(
        has_one = backend_operator @ EnclzError::Unauthorized,
    )]
    pub group_config: Box<Account<'info, GroupConfig>>,

    #[account(
        mut,
        seeds = [WALLET_SEED, group_config.key().as_ref(), &[agent_index]],
        bump = agent_wallet.bump,
        constraint = agent_wallet.group == group_config.key() @ EnclzError::Unauthorized,
    )]
    pub agent_wallet: Box<Account<'info, AgentWallet>>,

    #[account(
        mut,
        constraint = from_token_account.owner == agent_wallet.key() @ EnclzError::InvalidTokenAccount,
        constraint = from_token_account.mint == protocol_fee_token_account.mint @ EnclzError::InvalidMint,
    )]
    pub from_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub to_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), jupiter_program.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Box<Account<'info, WhitelistEntry>>,

    #[account(
        mut,
        constraint = protocol_fee_token_account.owner == group_config.protocol_fee_wallet @ EnclzError::InvalidFeeAccount,
    )]
    pub protocol_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: program is authorized via the type-2 whitelist_entry keyed on
    /// `jupiter_program.key()`; the seed binding plus entry_type assertion
    /// makes a non-whitelisted program unreachable.
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_execute_swap<'info>(
    context: Context<'info, ExecuteSwapAccountConstraints<'info>>,
    amount_in: u64,
    _minimum_amount_out: u64,
    expected_nonce: u64,
    agent_index: u8,
    route_data: Vec<u8>,
) -> Result<()> {
    require!(amount_in > 0, EnclzError::InvalidAmount);

    // Step 1: nonce check before any other state read.
    let agent_wallet = &mut context.accounts.agent_wallet;
    require!(
        expected_nonce == agent_wallet.operator_nonce,
        EnclzError::NonceMismatch
    );

    // Step 2: bump nonce. Rolls back with the rest of the transaction on failure.
    agent_wallet.operator_nonce = agent_wallet
        .operator_nonce
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    // Step 3: roll counters when the on-chain clock crossed a boundary.
    let now = Clock::get()?.unix_timestamp;
    if needs_daily_reset(agent_wallet.last_spend_reset, now) {
        agent_wallet.spent_today = 0;
        agent_wallet.last_spend_reset = now;
    }
    if needs_hourly_reset(agent_wallet.last_hour_reset, now) {
        agent_wallet.tx_count_this_hour = 0;
        agent_wallet.last_hour_reset = now;
    }

    // Steps 4–6: limits applied to gross `amount_in` (same as execute_transfer).
    require!(
        amount_in <= agent_wallet.per_tx_limit,
        EnclzError::PerTxLimitExceeded
    );
    let projected_spent = agent_wallet
        .spent_today
        .checked_add(amount_in)
        .ok_or(EnclzError::InvalidAmount)?;
    require!(
        projected_spent <= agent_wallet.daily_limit,
        EnclzError::DailyLimitExceeded
    );
    require!(
        agent_wallet.tx_count_this_hour < agent_wallet.hourly_tx_cap,
        EnclzError::HourlyCapExceeded
    );

    // Step 7: only type-2 (PROTOCOL) whitelist entries authorize a swap CPI.
    require!(
        context.accounts.whitelist_entry.entry_type == entry_type::PROTOCOL,
        EnclzError::WhitelistViolation
    );

    // Step 8: fee math.
    let (_net_amount_in, fee) = compute_fee(amount_in)?;

    // Step 9: PDA signer seeds for both the fee transfer and the Jupiter CPI.
    let group_key = context.accounts.group_config.key();
    let agent_bump = agent_wallet.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        WALLET_SEED,
        group_key.as_ref(),
        &[agent_index],
        &[agent_bump],
    ]];

    // Step 10: deduct protocol fee from the agent ATA BEFORE the swap. Output
    // mint may differ from input mint, so taking fee from input is the only
    // deterministic option — see design.md.
    if fee > 0 {
        let cpi_accounts = Transfer {
            from: context.accounts.from_token_account.to_account_info(),
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

    // Step 11: Jupiter v6 CPI. Route legs flow through `remaining_accounts`
    // because v6 accepts a variable account list shaped by the route plan.
    let mut metas: Vec<AccountMeta> = Vec::with_capacity(context.remaining_accounts.len());
    let mut infos: Vec<AccountInfo<'info>> =
        Vec::with_capacity(context.remaining_accounts.len() + 1);
    for account in context.remaining_accounts.iter() {
        metas.push(if account.is_writable {
            AccountMeta::new(*account.key, account.is_signer)
        } else {
            AccountMeta::new_readonly(*account.key, account.is_signer)
        });
        infos.push(account.clone());
    }
    infos.push(context.accounts.jupiter_program.to_account_info());

    let ix = Instruction {
        program_id: context.accounts.jupiter_program.key(),
        accounts: metas,
        data: route_data,
    };
    invoke_signed(&ix, &infos, signer_seeds)?;

    // Step 12: counters reflect gross amount_in (fee + net both count against
    // the daily cap; type-2 entries are uncapped so no amount_used update).
    agent_wallet.spent_today = projected_spent;
    agent_wallet.tx_count_this_hour = agent_wallet
        .tx_count_this_hour
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    Ok(())
}
