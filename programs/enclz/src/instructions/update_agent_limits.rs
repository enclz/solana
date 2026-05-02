use anchor_lang::prelude::*;

use crate::errors::EnclzError;
use crate::state::{AgentWallet, GroupConfig};

#[derive(Accounts)]
pub struct UpdateAgentLimitsAccountConstraints<'info> {
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,

    #[account(
        mut,
        constraint = agent_wallet.group == group_config.key() @ EnclzError::Unauthorized,
    )]
    pub agent_wallet: Account<'info, AgentWallet>,
}

pub fn handle_update_agent_limits(
    context: Context<UpdateAgentLimitsAccountConstraints>,
    daily_limit: Option<u64>,
    per_tx_limit: Option<u64>,
    hourly_tx_cap: Option<u8>,
) -> Result<()> {
    let agent_wallet = &mut context.accounts.agent_wallet;
    if let Some(value) = daily_limit {
        agent_wallet.daily_limit = value;
    }
    if let Some(value) = per_tx_limit {
        agent_wallet.per_tx_limit = value;
    }
    if let Some(value) = hourly_tx_cap {
        agent_wallet.hourly_tx_cap = value;
    }
    Ok(())
}
