mod common;

use common::{
    add_agent_instruction, add_to_whitelist_instruction, assert_anchor_error,
    execute_swap_instruction, provision_group_with_router, TestContext, STARTING_LAMPORTS,
};
use enclz::errors::EnclzError;
use enclz::state::whitelist_entry::entry_type;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const AGENT_NAME: [u8; 32] = *b"swap-bot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const PROTOCOL_LABEL: [u8; 32] = *b"jupiter-v6\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const EXTERNAL_LABEL: [u8; 32] = *b"merchant\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

#[allow(dead_code)]
struct Setup {
    context: TestContext,
    group_pda: Pubkey,
    backend_operator: Keypair,
    protocol_fee_wallet: Keypair,
    mint: Pubkey,
    mint_authority: Keypair,
    agent_pda: Pubkey,
    agent_token_account: Pubkey,
    protocol_fee_token_account: Pubkey,
}

fn setup_with_funded_agent(initial_balance: u64) -> Setup {
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
        mint_authority,
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
        EXTERNAL_LABEL,
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
fn execute_swap(
    setup: &mut Setup,
    operator: &Keypair,
    from_token_account: Pubkey,
    to_token_account: Pubkey,
    whitelist_entry: Pubkey,
    jupiter_program: Pubkey,
    amount_in: u64,
    expected_nonce: u64,
    route_data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> litesvm::types::TransactionResult {
    let instruction = execute_swap_instruction(
        &setup.context.program_id,
        &operator.pubkey(),
        &setup.group_pda,
        &setup.agent_pda,
        &from_token_account,
        &to_token_account,
        &whitelist_entry,
        &setup.protocol_fee_token_account,
        &jupiter_program,
        amount_in,
        0,
        expected_nonce,
        0,
        route_data,
        remaining_accounts,
    );
    let signer = operator.insecure_clone();
    setup.context.send_signed(instruction, &[&signer])
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn successful_swap_deducts_fee_invokes_jupiter_and_increments_counters() {
    let mut setup = setup_with_funded_agent(5_000_000);

    // Stub program acts as Jupiter v6: noop opcode (data == [0]) returns Ok
    // immediately. The test asserts on enclz-side state mutations only —
    // route legs are exercised in the Mocha + devnet integration tests.
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    // Output token account: a separate ATA owned by an unrelated keypair on
    // the same mint — the noop stub doesn't actually move tokens, so its
    // balance stays 0; we only need a writable account for Anchor.
    let output_owner = Keypair::new();
    setup.context.airdrop(&output_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&output_owner, &setup.mint, &output_owner.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(result.is_ok(), "swap should succeed: {result:?}");

    assert_eq!(
        setup.context.token_balance(&setup.protocol_fee_token_account),
        1_000,
        "10 bps fee on 1 USDC"
    );
    assert_eq!(
        setup.context.token_balance(&setup.agent_token_account),
        5_000_000 - 1_000,
        "fee deducted from agent ATA; net 999_000 stays put because noop stub doesn't move it"
    );

    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 1);
    assert_eq!(agent.spent_today, 1_000_000); // gross, not 999_000
    assert_eq!(agent.tx_count_this_hour, 1);
}

// ---------------------------------------------------------------------------
// Negative paths — no Jupiter CPI is reached, so jupiter_program can be any
// pubkey (we use Pubkey::new_unique()).
// ---------------------------------------------------------------------------

#[test]
fn non_type_2_whitelist_entry_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut setup, target, now + 86_400, 5_000_000);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    // The whitelist seed binds to `target`; pass the same pubkey as the
    // jupiter_program so seed validation passes — the entry_type check is
    // what should fail.
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        entry_pda,
        target,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::WhitelistViolation);
}

#[test]
fn stale_nonce_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        500_000,
        99,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::NonceMismatch);

    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 0);
    assert_eq!(agent.spent_today, 0);
}

#[test]
fn amount_in_above_per_tx_limit_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    // Default per_tx_limit is 1_000_000; 1_500_000 must reject.
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        1_500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::PerTxLimitExceeded);
}

#[test]
fn projected_spent_exceeds_daily_limit_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.spent_today = 9_500_000;
        agent.last_spend_reset = now;
    });

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::DailyLimitExceeded);
}

#[test]
fn hourly_cap_reached_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let now = setup.context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.tx_count_this_hour = agent.hourly_tx_cap;
        agent.last_hour_reset = now;
    });

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::HourlyCapExceeded);
}

#[test]
fn non_operator_signer_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    let attacker = Keypair::new();
    setup.context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);
    let from_ata = setup.agent_token_account;
    let result = execute_swap(
        &mut setup,
        &attacker,
        from_ata,
        to_ata,
        router_entry,
        stub_program_id,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::Unauthorized);
}

#[test]
fn from_token_account_owner_mismatch_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let to_owner = Keypair::new();
    setup.context.airdrop(&to_owner.pubkey(), STARTING_LAMPORTS);
    let to_ata = setup
        .context
        .create_ata(&to_owner, &setup.mint, &to_owner.pubkey());

    // ATA owned by a non-agent keypair.
    let bystander = Keypair::new();
    setup.context.airdrop(&bystander.pubkey(), STARTING_LAMPORTS);
    let bystander_ata = setup
        .context
        .create_ata(&bystander, &setup.mint, &bystander.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_swap(
        &mut setup,
        &operator,
        bystander_ata, // wrong from
        to_ata,
        router_entry,
        stub_program_id,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::InvalidTokenAccount);
}

// 3.9 — `fee + net == amount_in` for any amount_in is already covered by
// `util::fee::tests::fee_plus_net_always_equals_amount`, which exhaustively
// tests boundary values including u64::MAX. Re-expressing it at the LiteSVM
// layer would only add re-execution of compute_fee through the on-chain
// program path; the property is the same.
#[test]
fn fee_plus_net_property_pinned_by_lib_unit_test() {
    // Sanity-check the boundary cases here too so the property is documented
    // alongside the swap-instruction tests.
    use enclz::util::fee::compute_fee;
    for &amount in &[1u64, 100, 999, 1_000, 1_000_000, 10_050_000, u64::MAX / 2, u64::MAX] {
        let (net, fee) = compute_fee(amount).unwrap();
        assert_eq!(net.checked_add(fee), Some(amount), "amount = {amount}");
    }
}
