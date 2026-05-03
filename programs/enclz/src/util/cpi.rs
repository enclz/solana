use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};

/// Build and dispatch a CPI to an unchecked program with a variable account
/// list. Used by `execute_swap` and `execute_lending_op` to forward
/// `remaining_accounts` to Jupiter / lending programs whose account layouts are
/// shaped at runtime by the backend.
pub fn invoke_protocol_cpi<'info>(
    program: &UncheckedAccount<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    data: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut metas: Vec<AccountMeta> = Vec::with_capacity(remaining_accounts.len());
    let mut infos: Vec<AccountInfo<'info>> = Vec::with_capacity(remaining_accounts.len() + 1);
    for account in remaining_accounts.iter() {
        metas.push(if account.is_writable {
            AccountMeta::new(*account.key, account.is_signer)
        } else {
            AccountMeta::new_readonly(*account.key, account.is_signer)
        });
        infos.push(account.clone());
    }
    infos.push(program.to_account_info());

    let ix = Instruction {
        program_id: program.key(),
        accounts: metas,
        data,
    };
    invoke_signed(&ix, &infos, signer_seeds)?;
    Ok(())
}
