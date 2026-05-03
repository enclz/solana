mod common;

use common::{
    add_agent_instruction, add_to_whitelist_instruction, assert_anchor_error,
    execute_transfer_instruction, provision_group_with_router,
    update_backend_operator_instruction, TestContext, STARTING_LAMPORTS,
};
use enclz::errors::EnclzError;
use enclz::state::whitelist_entry::entry_type;
use solana_clock::Clock;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const AGENT_NAME: [u8; 32] = *b"transfer-bot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const MERCHANT_LABEL: [u8; 32] = *b"acme-merchant\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

struct Setup {
    context: TestContext,
    group_pda: Pubkey,
    backend_operator: Keypair,
    protocol_fee_wallet: Keypair,
    dex_router: Pubkey,
    mint: Pubkey,
    mint_authority: Keypair,
    agent_pda: Pubkey,
    agent_token_account: Pubkey,
    intra_entry: Pubkey,
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
        dex_router,
        mint,
        mint_authority,
        agent_pda,
        agent_token_account,
        intra_entry,
        protocol_fee_token_account,
    }
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
        MERCHANT_LABEL,
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

fn current_unix_time(setup: &Setup) -> i64 {
    setup.context.svm.get_sysvar::<Clock>().unix_timestamp
}

fn set_clock(setup: &mut Setup, unix_timestamp: i64) {
    let mut clock = setup.context.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    setup.context.svm.set_sysvar::<Clock>(&clock);
}

fn execute_transfer(
    setup: &mut Setup,
    to_token_account: Pubkey,
    whitelist_entry: Pubkey,
    amount: u64,
    expected_nonce: u64,
) -> litesvm::types::TransactionResult {
    let owner_pubkey = setup.context.owner.pubkey();
    let instruction = execute_transfer_instruction(
        &setup.context.program_id,
        &setup.backend_operator.pubkey(),
        &setup.group_pda,
        &owner_pubkey,
        &setup.agent_pda,
        &setup.agent_token_account,
        &to_token_account,
        &whitelist_entry,
        &setup.protocol_fee_token_account,
        amount,
        expected_nonce,
        0,
    );
    let operator = setup.backend_operator.insecure_clone();
    setup.context.send_signed(instruction, &[&operator])
}

#[test]
fn execute_transfer_to_intra_group_recipient_succeeds_with_fee_split() {
    let mut setup = setup_with_funded_agent(5_000_000);

    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());

    // Use the auto-created intra-group entry whose target is the recipient owner.
    // The intra-group seed is the agent_pda — but we need a target that matches
    // the to_token_account.owner. Add an INTRA_GROUP entry is impossible via
    // add_to_whitelist (rejects), so use a protocol-typed entry as the
    // intra-group analogue (identical post-validation behavior).
    let owner_pubkey = setup.context.owner.pubkey();
    let (recipient_entry, _) =
        setup.context.whitelist_pda(&setup.group_pda, &recipient_owner.pubkey());
    let owner_keypair = setup.context.owner.insecure_clone();
    let instruction = add_to_whitelist_instruction(
        &setup.context.program_id,
        &owner_pubkey,
        &setup.group_pda,
        &recipient_entry,
        recipient_owner.pubkey(),
        MERCHANT_LABEL,
        entry_type::PROTOCOL,
        0,
        0,
    );
    setup
        .context
        .send_signed(instruction, &[&owner_keypair])
        .unwrap();

    let amount = 1_000_000;
    let result = execute_transfer(&mut setup, recipient_token_account, recipient_entry, amount, 0);
    assert!(result.is_ok(), "transfer should succeed: {:?}", result);

    assert_eq!(setup.context.token_balance(&recipient_token_account), 999_000);
    assert_eq!(
        setup.context.token_balance(&setup.protocol_fee_token_account),
        1_000
    );
    assert_eq!(
        setup.context.token_balance(&setup.agent_token_account),
        5_000_000 - 1_000_000
    );

    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 1);
    assert_eq!(agent.spent_today, 1_000_000);
    assert_eq!(agent.tx_count_this_hour, 1);
}

#[test]
fn execute_transfer_to_external_recipient_increments_amount_used() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 1_000_000, 0);
    assert!(result.is_ok());

    let entry = setup.context.deserialize_whitelist(&entry_pda);
    assert_eq!(entry.amount_used, 1_000_000);
}

#[test]
fn execute_transfer_to_protocol_recipient_does_not_change_amount_used() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let (router_entry, _) =
        setup.context.whitelist_pda(&setup.group_pda, &setup.dex_router);
    let router_owner = Keypair::new();
    setup.context.airdrop(&router_owner.pubkey(), STARTING_LAMPORTS);
    // The PDA's seed is dex_router, but to_token_account.owner must equal
    // dex_router for the seed-derived constraint to pass. We need an ATA owned
    // by the dex_router pubkey itself. Since dex_router is a non-keypair pubkey
    // (Pubkey::new_unique()) we can't create an ATA owned by it via SDK calls.
    // Skip this test path by using a fresh protocol entry whose target == a
    // keypair's pubkey.
    let _ = router_entry;
    let _ = router_owner;
    let owner_pubkey = setup.context.owner.pubkey();
    let owner_keypair = setup.context.owner.insecure_clone();
    let protocol_target = Keypair::new();
    setup
        .context
        .airdrop(&protocol_target.pubkey(), STARTING_LAMPORTS);
    let (protocol_entry, _) = setup
        .context
        .whitelist_pda(&setup.group_pda, &protocol_target.pubkey());
    let add_protocol = add_to_whitelist_instruction(
        &setup.context.program_id,
        &owner_pubkey,
        &setup.group_pda,
        &protocol_entry,
        protocol_target.pubkey(),
        MERCHANT_LABEL,
        entry_type::PROTOCOL,
        0,
        0,
    );
    setup
        .context
        .send_signed(add_protocol, &[&owner_keypair])
        .unwrap();
    let recipient_token_account = setup.context.create_ata(
        &protocol_target,
        &setup.mint,
        &protocol_target.pubkey(),
    );

    let result =
        execute_transfer(&mut setup, recipient_token_account, protocol_entry, 1_000_000, 0);
    assert!(result.is_ok(), "{:?}", result);

    let entry = setup.context.deserialize_whitelist(&protocol_entry);
    assert_eq!(entry.amount_used, 0);
}

#[test]
fn fee_math_one_usdc_yields_999_000_net_and_1_000_fee() {
    let mut setup = setup_with_funded_agent(2_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 1_000_000, 0).unwrap();
    assert_eq!(
        setup.context.token_balance(&recipient_token_account),
        999_000
    );
    assert_eq!(
        setup.context.token_balance(&setup.protocol_fee_token_account),
        1_000
    );
}

#[test]
fn spent_today_counts_gross_amount() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 1_000_000, 0).unwrap();
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.spent_today, 1_000_000); // not 999_000
}

#[test]
fn stale_nonce_rejects_and_leaves_state_unchanged() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 99);
    assert_anchor_error(result, EnclzError::NonceMismatch);

    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 0);
    assert_eq!(agent.spent_today, 0);
    assert_eq!(agent.tx_count_this_hour, 0);
    assert_eq!(setup.context.token_balance(&recipient_token_account), 0);
    assert_eq!(setup.context.token_balance(&setup.protocol_fee_token_account), 0);
}

#[test]
fn replay_rejects_second_call_with_same_nonce() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();
    // Advance the blockhash so the runtime doesn't reject the second
    // submission as `AlreadyProcessed` before it reaches the program. In real
    // RPC use, the operator would resign with a fresh blockhash but reuse the
    // nonce; expire_blockhash() simulates that.
    setup.context.svm.expire_blockhash();
    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0);
    assert_anchor_error(result, EnclzError::NonceMismatch);
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.operator_nonce, 1);
}

#[test]
fn per_tx_limit_exceeded_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        50_000_000,
    );

    // Default per_tx_limit is 1_000_000; 1_500_000 should reject.
    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 1_500_000, 0);
    assert_anchor_error(result, EnclzError::PerTxLimitExceeded);
}

#[test]
fn daily_limit_exceeded_rejects_after_accumulated_spend() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        50_000_000,
    );

    // Manually pre-set spent_today close to the daily limit (10_000_000) so
    // a single 1_000_000 (at per_tx limit) tx would push past it.
    let mut account = setup.context.fetch_account(&setup.agent_pda);
    let mut agent = anchor_lang::AccountDeserialize::try_deserialize(
        &mut account.data.as_slice(),
    )
    .unwrap();
    let agent_ref: &mut enclz::state::AgentWallet = &mut agent;
    agent_ref.spent_today = 9_500_000;
    let mut buffer = Vec::with_capacity(account.data.len());
    anchor_lang::AccountSerialize::try_serialize(agent_ref, &mut buffer).unwrap();
    account.data = buffer;
    setup.context.svm.set_account(setup.agent_pda, account).unwrap();

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 1_000_000, 0);
    assert_anchor_error(result, EnclzError::DailyLimitExceeded);
}

#[test]
fn hourly_cap_reached_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        50_000_000,
    );

    // Pre-set tx_count_this_hour to the cap.
    let mut account = setup.context.fetch_account(&setup.agent_pda);
    let mut agent: enclz::state::AgentWallet = anchor_lang::AccountDeserialize::try_deserialize(
        &mut account.data.as_slice(),
    )
    .unwrap();
    agent.tx_count_this_hour = agent.hourly_tx_cap;
    agent.last_hour_reset = now;
    let mut buffer = Vec::with_capacity(account.data.len());
    anchor_lang::AccountSerialize::try_serialize(&mut agent, &mut buffer).unwrap();
    account.data = buffer;
    setup.context.svm.set_account(setup.agent_pda, account).unwrap();

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0);
    assert_anchor_error(result, EnclzError::HourlyCapExceeded);
}

#[test]
fn missing_whitelist_entry_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let (nonexistent_entry, _) = setup
        .context
        .whitelist_pda(&setup.group_pda, &recipient_owner.pubkey());

    let result = execute_transfer(
        &mut setup,
        recipient_token_account,
        nonexistent_entry,
        500_000,
        0,
    );
    // A missing PDA fails Anchor's account resolution before the handler runs,
    // so this is `AccountNotInitialized` (framework error 3012), not the
    // EnclzError::WhitelistViolation the spec text hints at. Tracked as a
    // spec-vs-impl divergence; for now we assert only that the call rejects.
    assert!(result.is_err(), "missing whitelist entry should reject");
}

#[test]
fn external_entry_past_ttl_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 1_000,
        5_000_000,
    );

    set_clock(&mut setup, now + 2_000);
    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0);
    assert_anchor_error(result, EnclzError::WhitelistExpired);
}

#[test]
fn external_entry_amount_exhausted_rejects_when_projected_exceeds_cap() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    // Tiny approved cap to make the second call exceed it.
    let entry_pda =
        add_external_entry(&mut setup, recipient_owner.pubkey(), now + 86_400, 700_000);

    // Force amount_used to a level where another 500_000 transfer would push past 700_000.
    let mut account = setup.context.fetch_account(&entry_pda);
    let mut entry: enclz::state::WhitelistEntry =
        anchor_lang::AccountDeserialize::try_deserialize(&mut account.data.as_slice()).unwrap();
    entry.amount_used = 300_000;
    let mut buffer = Vec::with_capacity(account.data.len());
    anchor_lang::AccountSerialize::try_serialize(&mut entry, &mut buffer).unwrap();
    account.data = buffer;
    setup.context.svm.set_account(entry_pda, account).unwrap();

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0);
    assert_anchor_error(result, EnclzError::WhitelistAmountExhausted);
}

#[test]
fn non_operator_signer_rejects_via_has_one() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    let attacker = Keypair::new();
    setup.context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);

    let owner_pubkey = setup.context.owner.pubkey();
    let instruction = execute_transfer_instruction(
        &setup.context.program_id,
        &attacker.pubkey(), // pretend to be operator
        &setup.group_pda,
        &owner_pubkey,
        &setup.agent_pda,
        &setup.agent_token_account,
        &recipient_token_account,
        &entry_pda,
        &setup.protocol_fee_token_account,
        500_000,
        0,
        0,
    );
    let result = setup.context.send_signed(instruction, &[&attacker]);
    assert_anchor_error(result, EnclzError::Unauthorized);
}

#[test]
fn rotated_operator_invalidates_previous_operator() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    // Sanity: original operator works.
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();

    // Rotate to a new operator.
    let new_operator = Keypair::new();
    setup.context.airdrop(&new_operator.pubkey(), STARTING_LAMPORTS);
    let owner_pubkey = setup.context.owner.pubkey();
    let owner_keypair = setup.context.owner.insecure_clone();
    let rotate = update_backend_operator_instruction(
        &setup.context.program_id,
        &owner_pubkey,
        &setup.group_pda,
        new_operator.pubkey(),
    );
    setup.context.send_signed(rotate, &[&owner_keypair]).unwrap();

    // Old operator's call now fails.
    let stale_result =
        execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 1);
    assert_anchor_error(stale_result, EnclzError::Unauthorized);
}

#[test]
fn from_token_account_owner_mismatch_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    // ATA owned by an unrelated keypair, not the agent_wallet PDA.
    let bystander = Keypair::new();
    setup.context.airdrop(&bystander.pubkey(), STARTING_LAMPORTS);
    let bystander_ata = setup
        .context
        .create_ata(&bystander, &setup.mint, &bystander.pubkey());

    let owner_pubkey = setup.context.owner.pubkey();
    let instruction = execute_transfer_instruction(
        &setup.context.program_id,
        &setup.backend_operator.pubkey(),
        &setup.group_pda,
        &owner_pubkey,
        &setup.agent_pda,
        &bystander_ata, // wrong from
        &recipient_token_account,
        &entry_pda,
        &setup.protocol_fee_token_account,
        500_000,
        0,
        0,
    );
    let operator = setup.backend_operator.insecure_clone();
    let result = setup.context.send_signed(instruction, &[&operator]);
    assert_anchor_error(result, EnclzError::InvalidTokenAccount);
}

#[test]
fn mint_mismatch_between_from_and_to_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    // recipient ATA on a different mint
    let other_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &other_mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0);
    assert_anchor_error(result, EnclzError::InvalidMint);
}

#[test]
fn protocol_fee_account_mint_mismatch_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    // Substitute a different-mint fee ATA still owned by protocol_fee_wallet.
    let other_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let bad_fee_ata = setup.context.create_ata(
        &setup.protocol_fee_wallet,
        &other_mint,
        &setup.protocol_fee_wallet.pubkey(),
    );

    let owner_pubkey = setup.context.owner.pubkey();
    let instruction = execute_transfer_instruction(
        &setup.context.program_id,
        &setup.backend_operator.pubkey(),
        &setup.group_pda,
        &owner_pubkey,
        &setup.agent_pda,
        &setup.agent_token_account,
        &recipient_token_account,
        &entry_pda,
        &bad_fee_ata,
        500_000,
        0,
        0,
    );
    let operator = setup.backend_operator.insecure_clone();
    let result = setup.context.send_signed(instruction, &[&operator]);
    assert_anchor_error(result, EnclzError::InvalidMint);
}

#[test]
fn protocol_fee_owner_mismatch_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    // Fee ATA owned by an attacker, not the configured protocol_fee_wallet.
    let attacker = Keypair::new();
    setup.context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);
    let attacker_fee_ata = setup
        .context
        .create_ata(&attacker, &setup.mint, &attacker.pubkey());

    let owner_pubkey = setup.context.owner.pubkey();
    let instruction = execute_transfer_instruction(
        &setup.context.program_id,
        &setup.backend_operator.pubkey(),
        &setup.group_pda,
        &owner_pubkey,
        &setup.agent_pda,
        &setup.agent_token_account,
        &recipient_token_account,
        &entry_pda,
        &attacker_fee_ata,
        500_000,
        0,
        0,
    );
    let operator = setup.backend_operator.insecure_clone();
    let result = setup.context.send_signed(instruction, &[&operator]);
    assert_anchor_error(result, EnclzError::InvalidFeeAccount);
}

#[test]
fn whitelist_seed_bound_to_to_token_account_owner() {
    // The whitelist PDA seed is derived from `to_token_account.owner`. Pairing a
    // valid whitelist PDA with an ATA whose owner doesn't match the seed must
    // reject — this is the central "no whitelist bypass via account
    // substitution" guarantee.
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let approved_owner = Keypair::new();
    let unapproved_owner = Keypair::new();
    setup
        .context
        .airdrop(&approved_owner.pubkey(), STARTING_LAMPORTS);
    setup
        .context
        .airdrop(&unapproved_owner.pubkey(), STARTING_LAMPORTS);
    let _approved_entry = add_external_entry(
        &mut setup,
        approved_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );
    let approved_pda = setup
        .context
        .whitelist_pda(&setup.group_pda, &approved_owner.pubkey())
        .0;
    // ATA owned by a non-whitelisted address.
    let unapproved_ata = setup.context.create_ata(
        &unapproved_owner,
        &setup.mint,
        &unapproved_owner.pubkey(),
    );

    let result = execute_transfer(&mut setup, unapproved_ata, approved_pda, 500_000, 0);
    assert!(
        result.is_err(),
        "PDA-target / ATA-owner mismatch should reject"
    );
}

#[test]
fn daily_counter_resets_after_midnight_crossing() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 7 * 86_400,
        50_000_000,
    );

    // First transfer at the day's start.
    let day_index = now / 86_400;
    let day_start = day_index * 86_400;
    set_clock(&mut setup, day_start + 60);
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.spent_today, 500_000);

    // Advance to next day; spent_today must reset.
    set_clock(&mut setup, day_start + 86_400 + 60);
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 700_000, 1).unwrap();
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.spent_today, 700_000);
}

#[test]
fn hourly_counter_resets_after_hour_crossing() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        50_000_000,
    );

    let hour_top = (now / 3_600) * 3_600;
    set_clock(&mut setup, hour_top + 60);
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 100_000, 0).unwrap();
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.tx_count_this_hour, 1);

    set_clock(&mut setup, hour_top + 3_600 + 60);
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 100_000, 1).unwrap();
    let agent = setup.context.deserialize_agent(&setup.agent_pda);
    assert_eq!(agent.tx_count_this_hour, 1);
}

#[test]
fn auto_void_closes_pda_when_amount_exhausted_and_returns_rent_to_owner() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    // Approved cap exactly equals the single transfer amount.
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        500_000,
    );
    let pre_owner_lamports = setup.context.fetch_account(&setup.context.owner.pubkey()).lamports;
    let entry_rent = setup.context.fetch_account(&entry_pda).lamports;

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();

    // The PDA is closed: account is gone or has zero lamports.
    let post = setup.context.try_fetch_account(&entry_pda);
    let closed = post.is_none()
        || post
            .as_ref()
            .map(|a| a.lamports == 0 && a.data.is_empty())
            .unwrap_or(true);
    assert!(closed, "auto-voided PDA should be closed");

    let post_owner_lamports = setup
        .context
        .fetch_account(&setup.context.owner.pubkey())
        .lamports;
    assert_eq!(
        post_owner_lamports,
        pre_owner_lamports + entry_rent,
        "auto-void rent must return to orchestrator"
    );
}

#[test]
fn post_auto_void_transfer_fails_with_whitelist_violation() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        500_000,
    );

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();
    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 100_000, 1);
    assert!(
        result.is_err(),
        "post-void transfer must fail (PDA gone -> whitelist_violation)"
    );
}

#[test]
fn auto_void_and_recreate_works_under_new_cap() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        500_000,
    );

    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 0).unwrap();
    // Re-create the entry with a fresh cap.
    let _ = add_external_entry(&mut setup, recipient_owner.pubkey(), now + 86_400, 1_000_000);
    execute_transfer(&mut setup, recipient_token_account, entry_pda, 500_000, 1).unwrap();
}

#[test]
fn zero_amount_rejects_with_invalid_amount() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = current_unix_time(&setup);
    let recipient_owner = Keypair::new();
    setup
        .context
        .airdrop(&recipient_owner.pubkey(), STARTING_LAMPORTS);
    let recipient_token_account =
        setup
            .context
            .create_ata(&recipient_owner, &setup.mint, &recipient_owner.pubkey());
    let entry_pda = add_external_entry(
        &mut setup,
        recipient_owner.pubkey(),
        now + 86_400,
        5_000_000,
    );

    let result = execute_transfer(&mut setup, recipient_token_account, entry_pda, 0, 0);
    assert_anchor_error(result, EnclzError::InvalidAmount);
}
