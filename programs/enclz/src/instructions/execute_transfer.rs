use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::{WALLET_SEED, WHITELIST_SEED};
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{AgentWallet, GroupConfig, WhitelistEntry};
use crate::util::fee::compute_fee;
use crate::util::time::{needs_daily_reset, needs_hourly_reset};

#[derive(Accounts)]
#[instruction(amount: u64, expected_nonce: u64, agent_index: u8)]
pub struct ExecuteTransferAccountConstraints<'info> {
    pub backend_operator: Signer<'info>,

    #[account(
        has_one = backend_operator @ EnclzError::Unauthorized,
    )]
    pub group_config: Box<Account<'info, GroupConfig>>,

    /// CHECK: pubkey is bound to `group_config.owner` via the address constraint.
    /// Receives rent lamports when an external whitelist entry auto-voids.
    #[account(
        mut,
        address = group_config.owner @ EnclzError::Unauthorized,
    )]
    pub group_owner: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [WALLET_SEED, group_config.key().as_ref(), &[agent_index]],
        bump = agent_wallet.bump,
        constraint = agent_wallet.group == group_config.key() @ EnclzError::Unauthorized,
    )]
    pub agent_wallet: Box<Account<'info, AgentWallet>>,

    #[account(
        mut,
        constraint = from_token_account.owner == agent_wallet.key() @ EnclzError::Unauthorized,
        constraint = from_token_account.mint == to_token_account.mint @ EnclzError::Unauthorized,
        constraint = from_token_account.mint == protocol_fee_token_account.mint @ EnclzError::Unauthorized,
    )]
    pub from_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub to_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), to_token_account.owner.as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Box<Account<'info, WhitelistEntry>>,

    #[account(
        mut,
        constraint = protocol_fee_token_account.owner == group_config.protocol_fee_wallet @ EnclzError::Unauthorized,
    )]
    pub protocol_fee_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_execute_transfer(
    context: Context<ExecuteTransferAccountConstraints>,
    amount: u64,
    expected_nonce: u64,
    agent_index: u8,
) -> Result<()> {
    require!(amount > 0, EnclzError::InvalidAmount);

    // Step 1: nonce check before any other state read so a stale tx cannot probe
    // whitelist or limit state.
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

    // Step 3: roll counters when the on-chain clock crossed the relevant boundary.
    let now = Clock::get()?.unix_timestamp;
    if needs_daily_reset(agent_wallet.last_spend_reset, now) {
        agent_wallet.spent_today = 0;
        agent_wallet.last_spend_reset = now;
    }
    if needs_hourly_reset(agent_wallet.last_hour_reset, now) {
        agent_wallet.tx_count_this_hour = 0;
        agent_wallet.last_hour_reset = now;
    }

    // Steps 4–6: limit checks.
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

    // Step 7: whitelist PDA existence is enforced by Anchor's seed constraint —
    // a missing PDA fails account resolution before the handler runs.
    // Step 8: type-1 (external recipient) extra checks.
    let entry_type_value = context.accounts.whitelist_entry.entry_type;
    let mut should_void = false;
    if entry_type_value == entry_type::EXTERNAL {
        let entry = &context.accounts.whitelist_entry;
        require!(now <= entry.ttl_expires_at, EnclzError::WhitelistExpired);
        let projected_used = entry
            .amount_used
            .checked_add(amount)
            .ok_or(EnclzError::InvalidAmount)?;
        require!(
            projected_used <= entry.approved_amount,
            EnclzError::WhitelistAmountExhausted
        );
        should_void = projected_used >= entry.approved_amount;
    }

    // Step 9: fee math.
    let (net, fee) = compute_fee(amount)?;

    // Step 10: two SPL `token::transfer` CPIs signed by the agent_wallet PDA.
    let group_key = context.accounts.group_config.key();
    let agent_bump = agent_wallet.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        WALLET_SEED,
        group_key.as_ref(),
        &[agent_index],
        &[agent_bump],
    ]];

    {
        let cpi_accounts = Transfer {
            from: context.accounts.from_token_account.to_account_info(),
            to: context.accounts.to_token_account.to_account_info(),
            authority: agent_wallet.to_account_info(),
        };
        let cpi_context = CpiContext::new_with_signer(
            context.accounts.token_program.key(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_context, net)?;
    }

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

    // Step 11: counters reflect gross amount (fee counts against the daily cap).
    agent_wallet.spent_today = projected_spent;
    agent_wallet.tx_count_this_hour = agent_wallet
        .tx_count_this_hour
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    // Step 12: external entries consume the approved cap; auto-void on exhaustion
    // so a subsequent transfer fails with WhitelistViolation rather than
    // WhitelistAmountExhausted (the PDA stops existing).
    if entry_type_value == entry_type::EXTERNAL {
        let entry = &mut context.accounts.whitelist_entry;
        entry.amount_used = entry
            .amount_used
            .checked_add(amount)
            .ok_or(EnclzError::InvalidAmount)?;

        if should_void {
            // Conditional close: Anchor's `close = receiver` runs unconditionally
            // at account resolution, so we invoke the AccountsClose trait
            // manually only when the cap has been consumed.
            context
                .accounts
                .whitelist_entry
                .close(context.accounts.group_owner.to_account_info())?;
        }
    }

    Ok(())
}
