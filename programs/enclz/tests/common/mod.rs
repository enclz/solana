#![allow(dead_code)]

use std::path::PathBuf;

use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::{AccountDeserialize, InstructionData};
use anchor_spl::associated_token::{get_associated_token_address, ID as ASSOCIATED_TOKEN_PROGRAM_ID};
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use enclz::constants::{GROUP_SEED, WALLET_SEED, WHITELIST_SEED};
use enclz::errors::EnclzError;
use enclz::state::{AgentWallet, GroupConfig, WhitelistEntry};
use litesvm::{types::TransactionResult, LiteSVM};
use litesvm_token::{spl_token, CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_account::Account;
use solana_instruction::{error::InstructionError, AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

pub const STARTING_LAMPORTS: u64 = 100_000_000_000;
pub const SYSTEM_PROGRAM_ID: Pubkey =
    anchor_lang::solana_program::system_program::ID;

/// Anchor maps `#[error_code]` variants to `InstructionError::Custom(6000 + index)`.
/// The backend matches errors by name, so reject-path tests must verify the
/// specific variant rather than just `is_err()`.
pub fn assert_anchor_error(result: TransactionResult, expected: EnclzError) {
    let failure = result.expect_err("expected transaction to fail");
    let expected_code = expected as u32 + anchor_lang::error::ERROR_CODE_OFFSET;
    match failure.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            assert_eq!(
                code, expected_code,
                "expected {expected:?} (code {expected_code}), got code {code}"
            );
        }
        other => panic!("expected Custom instruction error, got {other:?}"),
    }
}

pub struct TestContext {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
    pub owner: Keypair,
}

impl TestContext {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let program_id = enclz::ID;
        let mut so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        so_path.pop();
        so_path.pop();
        so_path.push("target/deploy/enclz.so");
        let program_bytes = std::fs::read(&so_path).unwrap_or_else(|error| {
            panic!(
                "expected built program at {} (run `anchor build` first): {error}",
                so_path.display()
            )
        });
        svm.add_program(program_id, &program_bytes).unwrap();
        let owner = Keypair::new();
        svm.airdrop(&owner.pubkey(), STARTING_LAMPORTS).unwrap();
        Self {
            svm,
            program_id,
            owner,
        }
    }

    /// Load the test_stubs program into LiteSVM under the given pubkey. Used by
    /// `execute_swap` / `execute_lending_op` happy-path tests where the stub
    /// stands in for Jupiter / Kamino. Called once per test that needs a real
    /// CPI target.
    pub fn add_stub_program(&mut self, program_id: &Pubkey) {
        let mut so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        so_path.pop();
        so_path.pop();
        so_path.push("target/deploy/test_stubs.so");
        let program_bytes = std::fs::read(&so_path).unwrap_or_else(|error| {
            panic!(
                "expected stub program at {} (run `cargo build-sbf --manifest-path test-programs/test_stubs/Cargo.toml`): {error}",
                so_path.display()
            )
        });
        self.svm.add_program(*program_id, &program_bytes).unwrap();
    }

    /// Read-modify-write helper for `AgentWallet`. Tests use this to seed
    /// preconditions like `spent_today` / `tx_count_this_hour` without going
    /// through `update_agent_limits`.
    pub fn rewrite_agent<F: FnOnce(&mut AgentWallet)>(&mut self, agent_pda: &Pubkey, mutate: F) {
        let mut account = self.fetch_account(agent_pda);
        let mut agent: AgentWallet =
            AgentWallet::try_deserialize(&mut account.data.as_slice()).unwrap();
        mutate(&mut agent);
        let mut buffer = Vec::with_capacity(account.data.len());
        anchor_lang::AccountSerialize::try_serialize(&mut agent, &mut buffer).unwrap();
        account.data = buffer;
        self.svm.set_account(*agent_pda, account).unwrap();
    }

    pub fn airdrop(&mut self, recipient: &Pubkey, lamports: u64) {
        self.svm.airdrop(recipient, lamports).unwrap();
    }

    pub fn group_pda(&self, owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[GROUP_SEED, owner.as_ref()], &self.program_id)
    }

    pub fn agent_pda(&self, group: &Pubkey, agent_index: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[WALLET_SEED, group.as_ref(), &[agent_index]],
            &self.program_id,
        )
    }

    pub fn whitelist_pda(&self, group: &Pubkey, target: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[WHITELIST_SEED, group.as_ref(), target.as_ref()],
            &self.program_id,
        )
    }

    pub fn fetch_account(&self, pubkey: &Pubkey) -> Account {
        self.svm
            .get_account(pubkey)
            .unwrap_or_else(|| panic!("account {pubkey} not found"))
    }

    pub fn try_fetch_account(&self, pubkey: &Pubkey) -> Option<Account> {
        self.svm.get_account(pubkey)
    }

    pub fn deserialize_group(&self, pubkey: &Pubkey) -> GroupConfig {
        let account = self.fetch_account(pubkey);
        GroupConfig::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn deserialize_agent(&self, pubkey: &Pubkey) -> AgentWallet {
        let account = self.fetch_account(pubkey);
        AgentWallet::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn deserialize_whitelist(&self, pubkey: &Pubkey) -> WhitelistEntry {
        let account = self.fetch_account(pubkey);
        WhitelistEntry::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn create_mint(&mut self, mint_authority: &Keypair, decimals: u8) -> Pubkey {
        CreateMint::new(&mut self.svm, mint_authority)
            .decimals(decimals)
            .send()
            .unwrap()
    }

    pub fn create_ata(&mut self, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
        CreateAssociatedTokenAccount::new(&mut self.svm, payer, mint)
            .owner(owner)
            .send()
            .unwrap()
    }

    pub fn mint_to(
        &mut self,
        mint_authority: &Keypair,
        mint: &Pubkey,
        token_account: &Pubkey,
        amount: u64,
    ) {
        MintTo::new(&mut self.svm, mint_authority, mint, token_account, amount)
            .send()
            .unwrap();
    }

    pub fn token_balance(&self, token_account: &Pubkey) -> u64 {
        let account = self.fetch_account(token_account);
        let token_account_state =
            spl_token::state::Account::unpack_from_slice(&account.data).unwrap();
        token_account_state.amount
    }

    pub fn associated_token_address(&self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, mint)
    }

    pub fn send_signed(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> TransactionResult {
        let payer = signers[0];
        let blockhash = self.svm.latest_blockhash();
        let message =
            Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
        let transaction = Transaction::new(signers, message, blockhash);
        self.svm.send_transaction(transaction)
    }
}

pub fn initialize_group_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    dex_router_entry: &Pubkey,
    group_name: [u8; 32],
    backend_operator: Pubkey,
    protocol_fee_wallet: Pubkey,
    dex_router: Pubkey,
) -> Instruction {
    let data = enclz::instruction::InitializeGroup {
        group_name,
        backend_operator,
        protocol_fee_wallet,
        dex_router,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*group_config, false),
            AccountMeta::new(*dex_router_entry, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn add_agent_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    agent_wallet: &Pubkey,
    intra_group_entry: &Pubkey,
    agent_token_account: &Pubkey,
    mint: &Pubkey,
    display_name: [u8; 32],
    daily_limit: Option<u64>,
    per_tx_limit: Option<u64>,
    hourly_tx_cap: Option<u8>,
) -> Instruction {
    let data = enclz::instruction::AddAgent {
        display_name,
        daily_limit,
        per_tx_limit,
        hourly_tx_cap,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*group_config, false),
            AccountMeta::new(*agent_wallet, false),
            AccountMeta::new(*intra_group_entry, false),
            AccountMeta::new(*agent_token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn update_agent_limits_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    agent_wallet: &Pubkey,
    daily_limit: Option<u64>,
    per_tx_limit: Option<u64>,
    hourly_tx_cap: Option<u8>,
) -> Instruction {
    let data = enclz::instruction::UpdateAgentLimits {
        daily_limit,
        per_tx_limit,
        hourly_tx_cap,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new(*agent_wallet, false),
        ],
        data,
    }
}

pub fn update_backend_operator_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    new_operator: Pubkey,
) -> Instruction {
    let data = enclz::instruction::UpdateBackendOperator { new_operator }.data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(*group_config, false),
        ],
        data,
    }
}

pub fn emergency_withdraw_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    agent_wallet: &Pubkey,
    agent_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    agent_index: u8,
) -> Instruction {
    let data = enclz::instruction::EmergencyWithdraw { agent_index }.data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new_readonly(*agent_wallet, false),
            AccountMeta::new(*agent_token_account, false),
            AccountMeta::new(*destination_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn add_to_whitelist_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    whitelist_entry: &Pubkey,
    target_address: Pubkey,
    label: [u8; 32],
    entry_type: u8,
    ttl_expires_at: i64,
    approved_amount: u64,
) -> Instruction {
    let data = enclz::instruction::AddToWhitelist {
        target_address,
        label,
        entry_type,
        ttl_expires_at,
        approved_amount,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new(*whitelist_entry, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn renew_whitelist_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    whitelist_entry: &Pubkey,
    target_address: Pubkey,
    ttl_expires_at: i64,
    approved_amount: u64,
) -> Instruction {
    let data = enclz::instruction::RenewWhitelistEntry {
        target_address,
        ttl_expires_at,
        approved_amount,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new(*whitelist_entry, false),
        ],
        data,
    }
}

pub fn remove_from_whitelist_instruction(
    program_id: &Pubkey,
    owner: &Pubkey,
    group_config: &Pubkey,
    whitelist_entry: &Pubkey,
    target_address: Pubkey,
) -> Instruction {
    let data = enclz::instruction::RemoveFromWhitelist { target_address }.data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new(*whitelist_entry, false),
        ],
        data,
    }
}

pub fn execute_transfer_instruction(
    program_id: &Pubkey,
    backend_operator: &Pubkey,
    group_config: &Pubkey,
    group_owner: &Pubkey,
    agent_wallet: &Pubkey,
    from_token_account: &Pubkey,
    to_token_account: &Pubkey,
    whitelist_entry: &Pubkey,
    protocol_fee_token_account: &Pubkey,
    amount: u64,
    expected_nonce: u64,
    agent_index: u8,
) -> Instruction {
    let data = enclz::instruction::ExecuteTransfer {
        amount,
        expected_nonce,
        agent_index,
    }
    .data();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*backend_operator, true),
            AccountMeta::new_readonly(*group_config, false),
            AccountMeta::new(*group_owner, false),
            AccountMeta::new(*agent_wallet, false),
            AccountMeta::new(*from_token_account, false),
            AccountMeta::new(*to_token_account, false),
            AccountMeta::new(*whitelist_entry, false),
            AccountMeta::new(*protocol_fee_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_swap_instruction(
    program_id: &Pubkey,
    backend_operator: &Pubkey,
    group_config: &Pubkey,
    agent_wallet: &Pubkey,
    from_token_account: &Pubkey,
    to_token_account: &Pubkey,
    whitelist_entry: &Pubkey,
    input_mint: &Pubkey,
    protocol_fee_token_account: &Pubkey,
    protocol_fee_wallet: &Pubkey,
    jupiter_program: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    expected_nonce: u64,
    agent_index: u8,
    route_data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let data = enclz::instruction::ExecuteSwap {
        amount_in,
        minimum_amount_out,
        expected_nonce,
        agent_index,
        route_data,
    }
    .data();
    let mut accounts = vec![
        AccountMeta::new(*backend_operator, true),
        AccountMeta::new_readonly(*group_config, false),
        AccountMeta::new(*agent_wallet, false),
        AccountMeta::new(*from_token_account, false),
        AccountMeta::new(*to_token_account, false),
        AccountMeta::new_readonly(*whitelist_entry, false),
        AccountMeta::new_readonly(*input_mint, false),
        AccountMeta::new(*protocol_fee_token_account, false),
        AccountMeta::new_readonly(*protocol_fee_wallet, false),
        AccountMeta::new_readonly(*jupiter_program, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    accounts.extend(remaining_accounts);
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

pub fn execute_lending_op_instruction(
    program_id: &Pubkey,
    backend_operator: &Pubkey,
    group_config: &Pubkey,
    agent_wallet: &Pubkey,
    agent_token_account: &Pubkey,
    whitelist_entry: &Pubkey,
    protocol_fee_token_account: &Pubkey,
    lending_program: &Pubkey,
    op_type: u8,
    amount: u64,
    expected_nonce: u64,
    agent_index: u8,
    cpi_data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let data = enclz::instruction::ExecuteLendingOp {
        op_type,
        amount,
        expected_nonce,
        agent_index,
        cpi_data,
    }
    .data();
    let mut accounts = vec![
        AccountMeta::new_readonly(*backend_operator, true),
        AccountMeta::new_readonly(*group_config, false),
        AccountMeta::new(*agent_wallet, false),
        AccountMeta::new(*agent_token_account, false),
        AccountMeta::new_readonly(*whitelist_entry, false),
        AccountMeta::new(*protocol_fee_token_account, false),
        AccountMeta::new_readonly(*lending_program, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    accounts.extend(remaining_accounts);
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

pub fn provision_group_with_router(
    context: &mut TestContext,
    backend_operator: Pubkey,
    protocol_fee_wallet: Pubkey,
    dex_router: Pubkey,
) -> Pubkey {
    let owner_pubkey = context.owner.pubkey();
    let (group_pda, _) = context.group_pda(&owner_pubkey);
    let (router_entry, _) = context.whitelist_pda(&group_pda, &dex_router);
    let instruction = initialize_group_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &router_entry,
        [0u8; 32],
        backend_operator,
        protocol_fee_wallet,
        dex_router,
    );
    let owner_keypair = context.owner.insecure_clone();
    context
        .send_signed(instruction, &[&owner_keypair])
        .expect("initialize_group should succeed");
    group_pda
}
