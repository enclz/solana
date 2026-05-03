mod common;

use common::{
    add_agent_instruction, add_to_whitelist_instruction, assert_anchor_error,
    execute_lending_op_instruction, provision_group_with_router, TestContext, STARTING_LAMPORTS,
};
use enclz::errors::EnclzError;
use enclz::state::whitelist_entry::entry_type;
use litesvm_token::spl_token;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const AGENT_NAME: [u8; 32] = *b"yield-bot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const PROTOCOL_LABEL: [u8; 32] = *b"kamino\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

const STUB_AUTH_SEED: &[u8] = b"stub-auth";

const OP_DEPOSIT: u8 = 0;
const OP_WITHDRAW: u8 = 1;

#[allow(dead_code)]
struct Setup {
    context: TestContext,
    group_pda: Pubkey,
    backend_operator: Keypair,
    protocol_fee_wallet: Keypair,
    mint: Pubkey,
    /// Set when the test mint's authority is the stub PDA (so the stub can
    /// mint tokens to the agent ATA during withdraw). `None` means the mint
    /// uses an ordinary keypair authority.
    stub_mint_authority: Option<Pubkey>,
    agent_pda: Pubkey,
    agent_token_account: Pubkey,
    protocol_fee_token_account: Pubkey,
}

/// Group + agent + agent ATA under a keypair-controlled mint. Used by every
/// test that doesn't need the stub to mint tokens during the lending CPI.
fn setup_with_keypair_mint(initial_balance: u64) -> Setup {
    let mut context = TestContext::new();
    let backend_operator = Keypair::new();
    let protocol_fee_wallet = Keypair::new();
    context.airdrop(&backend_operator.pubkey(), STARTING_LAMPORTS);
    context.airdrop(&protocol_fee_wallet.pubkey(), STARTING_LAMPORTS);
    let dex_router = Pubkey::new_unique();
    let group_pda = provision_group_with_router(
        &mut context,
        backend_operator.pubkey(),
        protocol_fee_wallet.pubkey(),
        dex_router,
    );

    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let (agent_pda, _) = context.agent_pda(&group_pda, 0);
    let (intra_entry, _) = context.whitelist_pda(&group_pda, &agent_pda);
    let agent_token_account = context.associated_token_address(&agent_pda, &mint);
    let add_agent = add_agent_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &intra_entry,
        &agent_token_account,
        &mint,
        AGENT_NAME,
        None,
        None,
        None,
    );
    context.send_signed(add_agent, &[&owner_keypair]).unwrap();

    if initial_balance > 0 {
        context.mint_to(&mint_authority, &mint, &agent_token_account, initial_balance);
    }

    let protocol_fee_token_account =
        context.create_ata(&protocol_fee_wallet, &mint, &protocol_fee_wallet.pubkey());

    Setup {
        context,
        group_pda,
        backend_operator,
        protocol_fee_wallet,
        mint,
        stub_mint_authority: None,
        agent_pda,
        agent_token_account,
        protocol_fee_token_account,
    }
}

/// Group + agent + agent ATA where the test mint's MintTokens authority is
/// the stub PDA. The withdraw happy-path test uses this so the stub can
/// credit redeemed tokens to the agent ATA via SPL `MintTo` during the
/// lending CPI. `initial_balance` is pre-minted via the temporary keypair
/// authority before the SetAuthority hand-off.
fn setup_with_stub_mint(stub_program_id: &Pubkey, initial_balance: u64) -> Setup {
    let mut context = TestContext::new();
    context.add_stub_program(stub_program_id);
    let backend_operator = Keypair::new();
    let protocol_fee_wallet = Keypair::new();
    context.airdrop(&backend_operator.pubkey(), STARTING_LAMPORTS);
    context.airdrop(&protocol_fee_wallet.pubkey(), STARTING_LAMPORTS);
    let dex_router = Pubkey::new_unique();
    let group_pda = provision_group_with_router(
        &mut context,
        backend_operator.pubkey(),
        protocol_fee_wallet.pubkey(),
        dex_router,
    );

    let temp_authority = Keypair::new();
    context.airdrop(&temp_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&temp_authority, 6);

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let (agent_pda, _) = context.agent_pda(&group_pda, 0);
    let (intra_entry, _) = context.whitelist_pda(&group_pda, &agent_pda);
    let agent_token_account = context.associated_token_address(&agent_pda, &mint);
    let add_agent = add_agent_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &intra_entry,
        &agent_token_account,
        &mint,
        AGENT_NAME,
        None,
        None,
        None,
    );
    context.send_signed(add_agent, &[&owner_keypair]).unwrap();

    if initial_balance > 0 {
        context.mint_to(&temp_authority, &mint, &agent_token_account, initial_balance);
    }

    // Hand off MintTokens authority to the stub PDA.
    let (stub_auth, _) = Pubkey::find_program_address(&[STUB_AUTH_SEED], stub_program_id);
    let set_authority_ix = spl_token::instruction::set_authority(
        &spl_token::id(),
        &mint,
        Some(&stub_auth),
        spl_token::instruction::AuthorityType::MintTokens,
        &temp_authority.pubkey(),
        &[&temp_authority.pubkey()],
    )
    .unwrap();
    let temp_clone = temp_authority.insecure_clone();
    context.send_signed(set_authority_ix, &[&temp_clone]).unwrap();

    let protocol_fee_token_account =
        context.create_ata(&protocol_fee_wallet, &mint, &protocol_fee_wallet.pubkey());

    Setup {
        context,
        group_pda,
        backend_operator,
        protocol_fee_wallet,
        mint,
        stub_mint_authority: Some(stub_auth),
        agent_pda,
        agent_token_account,
        protocol_fee_token_account,
    }
}

fn add_protocol_entry(setup: &mut Setup, target: Pubkey, label: [u8; 32]) -> Pubkey {
    let owner_pubkey = setup.context.owner.pubkey();
    let (entry_pda, _) = setup.context.whitelist_pda(&setup.group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &setup.context.program_id,
        &owner_pubkey,
        &setup.group_pda,
        &entry_pda,
        target,
        label,
        entry_type::PROTOCOL,
        0,
        0,
    );
    let owner_keypair = setup.context.owner.insecure_clone();
    setup
        .context
        .send_signed(instruction, &[&owner_keypair])
        .unwrap();
    entry_pda
}

fn add_external_entry(
    setup: &mut Setup,
    target: Pubkey,
    ttl_expires_at: i64,
    approved_amount: u64,
) -> Pubkey {
    let owner_pubkey = setup.context.owner.pubkey();
    let (entry_pda, _) = setup.context.whitelist_pda(&setup.group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &setup.context.program_id,
        &owner_pubkey,
        &setup.group_pda,
        &entry_pda,
        target,
        PROTOCOL_LABEL,
        entry_type::EXTERNAL,
        ttl_expires_at,
        approved_amount,
    );
    let owner_keypair = setup.context.owner.insecure_clone();
    setup
        .context
        .send_signed(instruction, &[&owner_keypair])
        .unwrap();
    entry_pda
}

#[allow(clippy::too_many_arguments)]
fn execute_lending_op(
    setup: &mut Setup,
    operator: &Keypair,
    whitelist_entry: Pubkey,
    lending_program: Pubkey,
    op_type: u8,
    amount: u64,
    expected_nonce: u64,
    cpi_data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> litesvm::types::TransactionResult {
    let instruction = execute_lending_op_instruction(
        &setup.context.program_id,
        &operator.pubkey(),
        &setup.group_pda,
        &setup.agent_pda,
        &setup.agent_token_account,
        &whitelist_entry,
        &setup.protocol_fee_token_account,
        &lending_program,
        op_type,
        amount,
        expected_nonce,
        0,
        cpi_data,
        remaining_accounts,
    );
    let signer = operator.insecure_clone();
    setup.context.send_signed(instruction, &[&signer])
}

// 4.1 — deposit happy path
#[test]
fn successful_deposit_deducts_fee_invokes_lending_and_increments_counters() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_DEPOSIT,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(result.is_ok(), "deposit should succeed: {result:?}");

    assert_eq!(
        setup.context.token_balance(&setup.protocol_fee_token_account),
        1_000
    );
    // Net 999_000 stays in the agent ATA — the noop stub doesn't actually
    // move it into a lending vault. The test asserts only enclz-side state.
    assert_eq!(
        setup.context.token_balance(&setup.agent_token_account),
        4_999_000
    );
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 1);
    assert_eq!(agent.spent_today, 1_000_000);
    assert_eq!(agent.tx_count_this_hour, 1);
}

// 4.2 — withdraw happy path. Stub mints `redeemed` to the agent ATA.
#[test]
fn successful_withdraw_deducts_fee_after_redeem_and_increments_counters() {
    let stub_program_id = Pubkey::new_unique();
    let mut setup = setup_with_stub_mint(&stub_program_id, 0);
    let stub_auth = setup.stub_mint_authority.expect("stub mint authority set");
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let redeemed: u64 = 1_000_000;
    let mut cpi_data = vec![1u8];
    cpi_data.extend_from_slice(&redeemed.to_le_bytes());

    let token_program_id = anchor_spl::token::ID;
    let remaining = vec![
        AccountMeta::new_readonly(token_program_id, false),
        AccountMeta::new(setup.mint, false),
        AccountMeta::new(setup.agent_token_account, false),
        AccountMeta::new_readonly(stub_auth, false),
    ];

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_WITHDRAW,
        redeemed,
        0,
        cpi_data,
        remaining,
    );
    assert!(result.is_ok(), "withdraw should succeed: {result:?}");

    assert_eq!(
        setup.context.token_balance(&setup.protocol_fee_token_account),
        1_000
    );
    assert_eq!(
        setup.context.token_balance(&setup.agent_token_account),
        999_000
    );
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 1);
    assert_eq!(agent.spent_today, redeemed);
    assert_eq!(agent.tx_count_this_hour, 1);
}

// 4.3 — non-type-2 whitelist rejected
#[test]
fn non_type_2_whitelist_entry_rejects() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let lending_target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut setup, lending_target, now + 86_400, 5_000_000);

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        entry_pda,
        lending_target,
        OP_DEPOSIT,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::WhitelistViolation);
}

// 4.4 — unknown op_type rejected
#[test]
fn unknown_op_type_rejects_with_invalid_amount() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        99,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::InvalidAmount);
}

// 4.5 — stale nonce rejected
#[test]
fn stale_nonce_rejects() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_DEPOSIT,
        500_000,
        99,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::NonceMismatch);
}

// 4.6 — daily limit enforced
#[test]
fn daily_limit_exceeded_rejects() {
    let mut setup = setup_with_keypair_mint(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.spent_today = 9_500_000;
        agent.last_spend_reset = now;
    });

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_DEPOSIT,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::DailyLimitExceeded);
}

// 4.7 — hourly cap enforced
#[test]
fn hourly_cap_reached_rejects() {
    let mut setup = setup_with_keypair_mint(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.tx_count_this_hour = agent.hourly_tx_cap;
        agent.last_hour_reset = now;
    });

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_DEPOSIT,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::HourlyCapExceeded);
}

// 4.8 — withdraw where the lending CPI did not credit any tokens.
// The spec text "redeemed_amount < protocol_fee" is unreachable mathematically
// (fee = amount * 10/10000 ≤ amount for any amount), so the realistic
// failure surface is the zero-redeemed case: post == pre, redeemed == 0, the
// require!(redeemed > 0) fires → InvalidAmount.
#[test]
fn withdraw_with_zero_redeemed_rejects_invalid_amount() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_lending_op(
        &mut setup,
        &operator,
        lending_entry,
        stub_program_id,
        OP_WITHDRAW,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::InvalidAmount);
}

// 4.9 — non-operator signer rejected
#[test]
fn non_operator_signer_rejects() {
    let mut setup = setup_with_keypair_mint(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let lending_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let attacker = Keypair::new();
    setup.context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);
    let result = execute_lending_op(
        &mut setup,
        &attacker,
        lending_entry,
        stub_program_id,
        OP_DEPOSIT,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::Unauthorized);
}
