// Stub program for LiteSVM tests of `execute_swap` and `execute_lending_op`.
// Stands in for Jupiter v6 / Kamino. Two opcodes:
//   data == [] or data[0] == 0  → noop, returns Ok (used for swap + deposit
//                                  happy paths where the test only verifies
//                                  spend-policy state mutations on the enclz
//                                  side and does not assert post-CPI token
//                                  balances).
//   data[0] == 1                  → "redeem": mints `u64::from_le_bytes(data[1..9])`
//                                  tokens into the destination ATA. Accounts:
//                                  [token_program, mint, dest_ata, mint_authority_pda].
//                                  The mint authority must be the PDA derived
//                                  from seed `b"stub-auth"` (the stub signs
//                                  the SPL `MintTo` CPI itself).

use anchor_lang::declare_id;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

declare_id!("4PhEhEZuZbQTC7WpKS6yMoRV6ySmpXVXUvPHQ624XQDU");

const STUB_AUTH_SEED: &[u8] = b"stub-auth";
const SPL_TOKEN_MINT_TO_DISCRIMINANT: u8 = 7;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.is_empty() || data[0] == 0 {
        return Ok(());
    }
    if data[0] != 1 {
        return Err(ProgramError::InvalidInstructionData);
    }
    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());

    let token_program = accounts.first().ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    let dest = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts.get(3).ok_or(ProgramError::NotEnoughAccountKeys)?;

    let (auth_pda, bump) = Pubkey::find_program_address(&[STUB_AUTH_SEED], program_id);
    if *authority.key != auth_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut ix_data = Vec::with_capacity(9);
    ix_data.push(SPL_TOKEN_MINT_TO_DISCRIMINANT);
    ix_data.extend_from_slice(&amount.to_le_bytes());

    let mint_to_ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*mint.key, false),
            AccountMeta::new(*dest.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: ix_data,
    };
    invoke_signed(
        &mint_to_ix,
        &[
            mint.clone(),
            dest.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        &[&[STUB_AUTH_SEED, &[bump]]],
    )
}

pub fn stub_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STUB_AUTH_SEED], program_id)
}
