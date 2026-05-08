use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{WALLET_SEED, WHITELIST_SEED};
use crate::errors::EnclzError;
use crate::state::whitelist_entry::entry_type;
use crate::state::{AgentWallet, GroupConfig, WhitelistEntry};
use crate::util::cpi::invoke_protocol_cpi;
use crate::util::fee::compute_fee;
use crate::util::time::needs_hourly_reset;

// `agent_index` reconstructs the agent_wallet PDA seed for the SPL transfer
// CPI signer (same convention as `execute_transfer`). `route_data` is the raw
// Jupiter v6 instruction payload; the backend constructs it with
// `net_amount_in = amount_in - protocol_fee` and the desired slippage so
// Jupiter's own `minimum_amount_out` check applies to the post-fee net.
//
// Mint policy: input/output mints are NOT pinned to `agent_wallet.mint`. The
// load-bearing safety constraint is `to_token_account.owner == agent_wallet`
// PDA — swap proceeds always remain in agent custody. A compromised operator
// can rotate holdings between mints but cannot exfiltrate them.
#[derive(Accounts)]
#[instruction(amount_in: u64, minimum_amount_out: u64, expected_nonce: u64, agent_index: u8)]
pub struct ExecuteSwapAccountConstraints<'info> {
    #[account(mut)]
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
        constraint = from_token_account.owner == agent_wallet.key() @ EnclzError::InvalidTokenAccount,
        constraint = from_token_account.mint == input_mint.key() @ EnclzError::InvalidMint,
    )]
    pub from_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = to_token_account.owner == agent_wallet.key() @ EnclzError::InvalidTokenAccount,
    )]
    pub to_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [WHITELIST_SEED, group_config.key().as_ref(), jupiter_program.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Box<Account<'info, WhitelistEntry>>,

    pub input_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = backend_operator,
        associated_token::mint = input_mint,
        associated_token::authority = protocol_fee_wallet,
    )]
    pub protocol_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: address-bound to `group_config.protocol_fee_wallet`; only used
    /// as the authority for the fee ATA derivation, so no data is read.
    #[account(
        address = group_config.protocol_fee_wallet @ EnclzError::InvalidFeeAccount,
    )]
    pub protocol_fee_wallet: UncheckedAccount<'info>,

    /// CHECK: program is authorized via the type-2 whitelist_entry keyed on
    /// `jupiter_program.key()`; the seed binding plus entry_type assertion
    /// makes a non-whitelisted program unreachable.
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_execute_swap<'info>(
    context: Context<'info, ExecuteSwapAccountConstraints<'info>>,
    amount_in: u64,
    // minimum_amount_out is part of the IDL ABI and surfaced to the backend,
    // but not validated here. Jupiter enforces it inside the CPI via route_data.
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

    // Step 3: roll the hourly counter when the on-chain clock crossed the hour
    // boundary. `spent_today` and `last_spend_reset` are deliberately not
    // touched on the swap path — daily and per-tx limits are mint-relative
    // and meaningless when applied to an arbitrary swap input mint.
    let now = Clock::get()?.unix_timestamp;
    if needs_hourly_reset(agent_wallet.last_hour_reset, now) {
        agent_wallet.tx_count_this_hour = 0;
        agent_wallet.last_hour_reset = now;
    }

    // Step 4: only the hourly transaction cap gates swaps. Funds-stay-in-custody
    // (enforced by `to_token_account.owner == agent_wallet`) removes the theft
    // threat that the daily and per-tx limits guarded against.
    require!(
        agent_wallet.tx_count_this_hour < agent_wallet.hourly_tx_cap,
        EnclzError::HourlyCapExceeded
    );

    // Step 5: only type-2 (PROTOCOL) whitelist entries authorize a swap CPI.
    require!(
        context.accounts.whitelist_entry.entry_type == entry_type::PROTOCOL,
        EnclzError::WhitelistViolation
    );

    // Step 6: fee math.
    let (_net_amount_in, fee) = compute_fee(amount_in)?;

    // Step 7: PDA signer seeds for both the fee transfer and the Jupiter CPI.
    let group_key = context.accounts.group_config.key();
    let agent_bump = agent_wallet.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        WALLET_SEED,
        group_key.as_ref(),
        &[agent_index],
        &[agent_bump],
    ]];

    // Step 8: deduct protocol fee from the agent ATA BEFORE the swap. Output
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

    // Step 9: Jupiter v6 CPI. Route legs flow through `remaining_accounts`
    // because v6 accepts a variable account list shaped by the route plan.
    invoke_protocol_cpi(
        &context.accounts.jupiter_program,
        context.remaining_accounts,
        route_data,
        signer_seeds,
    )?;

    // Step 10: only the hourly counter advances. `spent_today` stays untouched.
    agent_wallet.tx_count_this_hour = agent_wallet
        .tx_count_this_hour
        .checked_add(1)
        .ok_or(EnclzError::InvalidAmount)?;

    Ok(())
}
