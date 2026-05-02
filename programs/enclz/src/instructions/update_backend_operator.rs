use anchor_lang::prelude::*;

use crate::state::GroupConfig;

#[derive(Accounts)]
pub struct UpdateBackendOperatorAccountConstraints<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = owner,
    )]
    pub group_config: Account<'info, GroupConfig>,
}

pub fn handle_update_backend_operator(
    context: Context<UpdateBackendOperatorAccountConstraints>,
    new_operator: Pubkey,
) -> Result<()> {
    context.accounts.group_config.backend_operator = new_operator;
    Ok(())
}
