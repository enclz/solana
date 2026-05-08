mod common;

use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::{AccountDeserialize, AccountSerialize};
use common::{
    add_agent_instruction, add_to_whitelist_instruction, emergency_withdraw_instruction,
    initialize_group_instruction, provision_group_with_router, remove_from_whitelist_instruction,
    renew_whitelist_instruction, update_agent_limits_instruction,
    update_backend_operator_instruction, TestContext, STARTING_LAMPORTS,
};
use enclz::constants::{DEFAULT_DAILY_LIMIT, DEFAULT_HOURLY_CAP, DEFAULT_PER_TX_LIMIT};
use enclz::state::whitelist_entry::entry_type;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const DISPLAY_NAME: [u8; 32] = *b"agent-zero\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const SECOND_NAME: [u8; 32] = *b"agent-one\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
const MERCHANT_LABEL: [u8; 32] = *b"acme-merchant\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

fn unique_keys() -> (Pubkey, Pubkey, Pubkey) {
    (
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    )
}

#[test]
fn initialize_group_happy_path_stores_fields_and_router_entry() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let owner_pubkey = context.owner.pubkey();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let group = context.deserialize_group(&group_pda);
    assert_eq!(group.owner, owner_pubkey);
    assert_eq!(group.backend_operator, backend_operator);
    assert_eq!(group.protocol_fee_wallet, protocol_fee_wallet);
    assert_eq!(group.agent_count, 0);
    assert_eq!(group.group_name, [0u8; 32]);

    let (router_entry, _) = context.whitelist_pda(&group_pda, &dex_router);
    let entry = context.deserialize_whitelist(&router_entry);
    assert_eq!(entry.entry_type, entry_type::PROTOCOL);
    assert_eq!(entry.ttl_expires_at, 0);
    assert_eq!(entry.approved_amount, 0);
    assert_eq!(entry.added_by, owner_pubkey);
}

#[test]
fn initialize_group_stores_group_name_verbatim_including_non_utf8_bytes() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let owner_pubkey = context.owner.pubkey();
    let (group_pda, _) = context.group_pda(&owner_pubkey);
    let (router_entry, _) = context.whitelist_pda(&group_pda, &dex_router);
    let group_name: [u8; 32] = [0xFFu8; 32];
    let instruction = initialize_group_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &router_entry,
        group_name,
        backend_operator,
        protocol_fee_wallet,
        dex_router,
    );
    let owner_keypair = context.owner.insecure_clone();
    context
        .send_signed(instruction, &[&owner_keypair])
        .expect("initialize_group with non-UTF-8 name should succeed");

    let group = context.deserialize_group(&group_pda);
    assert_eq!(group.group_name, group_name);
}

#[test]
fn initialize_group_rejects_duplicate() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

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
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "expected duplicate init to fail");
}

fn add_first_agent(
    context: &mut TestContext,
    group_pda: Pubkey,
    mint: Pubkey,
    daily_limit: Option<u64>,
    per_tx_limit: Option<u64>,
    hourly_tx_cap: Option<u8>,
) -> (Pubkey, Pubkey, Pubkey) {
    let owner_pubkey = context.owner.pubkey();
    let (agent_pda, _) = context.agent_pda(&group_pda, 0);
    let (intra_entry, _) = context.whitelist_pda(&group_pda, &agent_pda);
    let agent_token_account = context.associated_token_address(&agent_pda, &mint);
    let instruction = add_agent_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &intra_entry,
        &agent_token_account,
        &mint,
        DISPLAY_NAME,
        daily_limit,
        per_tx_limit,
        hourly_tx_cap,
    );
    let owner_keypair = context.owner.insecure_clone();
    context
        .send_signed(instruction, &[&owner_keypair])
        .expect("add_agent should succeed");
    (agent_pda, intra_entry, agent_token_account)
}

#[test]
fn add_agent_with_defaults_applies_template_values() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let (agent_pda, _, agent_token_account) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let agent = context.deserialize_agent(&agent_pda);
    assert_eq!(agent.daily_limit, DEFAULT_DAILY_LIMIT);
    assert_eq!(agent.per_tx_limit, DEFAULT_PER_TX_LIMIT);
    assert_eq!(agent.hourly_tx_cap, DEFAULT_HOURLY_CAP);
    assert_eq!(agent.group, group_pda);
    assert_eq!(agent.mint, mint);
    assert_eq!(agent.display_name, DISPLAY_NAME);
    assert_eq!(agent.operator_nonce, 0);

    let group = context.deserialize_group(&group_pda);
    assert_eq!(group.agent_count, 1);

    assert_eq!(context.token_balance(&agent_token_account), 0);
}

#[test]
fn add_agent_overrides_applied_when_some() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let (agent_pda, _, _) = add_first_agent(
        &mut context,
        group_pda,
        mint,
        Some(50_000_000),
        Some(7_500_000),
        Some(20),
    );
    let agent = context.deserialize_agent(&agent_pda);
    assert_eq!(agent.daily_limit, 50_000_000);
    assert_eq!(agent.per_tx_limit, 7_500_000);
    assert_eq!(agent.hourly_tx_cap, 20);
}

#[test]
fn add_agent_creates_intra_group_whitelist_entry() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let (_agent_pda, intra_entry, _) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let entry = context.deserialize_whitelist(&intra_entry);
    assert_eq!(entry.entry_type, entry_type::INTRA_GROUP);
    assert_eq!(entry.ttl_expires_at, 0);
    assert_eq!(entry.approved_amount, 0);
    assert_eq!(entry.amount_used, 0);
}

#[test]
fn add_agent_creates_ata_owned_by_pda() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let (agent_pda, _, agent_token_account) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let token_account = context.fetch_account(&agent_token_account);
    let parsed = litesvm_token::spl_token::state::Account::unpack(&token_account.data)
        .unwrap();
    assert_eq!(parsed.owner, agent_pda);
    assert_eq!(parsed.mint, mint);
}

#[test]
fn add_agent_rejects_non_owner_signer() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let attacker = Keypair::new();
    context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);

    let (agent_pda, _) = context.agent_pda(&group_pda, 0);
    let (intra_entry, _) = context.whitelist_pda(&group_pda, &agent_pda);
    let agent_token_account = context.associated_token_address(&agent_pda, &mint);
    let instruction = add_agent_instruction(
        &context.program_id,
        &attacker.pubkey(),
        &group_pda,
        &agent_pda,
        &intra_entry,
        &agent_token_account,
        &mint,
        DISPLAY_NAME,
        None,
        None,
        None,
    );
    let result = context.send_signed(instruction, &[&attacker]);
    assert!(result.is_err(), "non-owner add_agent must fail");
}

#[test]
fn update_agent_limits_patches_only_some_fields() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);
    let (agent_pda, _, _) = add_first_agent(&mut context, group_pda, mint, None, None, None);

    let owner_pubkey = context.owner.pubkey();
    let instruction = update_agent_limits_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        Some(5_000_000),
        None,
        None,
    );
    let owner_keypair = context.owner.insecure_clone();
    context.send_signed(instruction, &[&owner_keypair]).unwrap();

    let agent = context.deserialize_agent(&agent_pda);
    assert_eq!(agent.daily_limit, 5_000_000);
    assert_eq!(agent.per_tx_limit, DEFAULT_PER_TX_LIMIT);
    assert_eq!(agent.hourly_tx_cap, DEFAULT_HOURLY_CAP);
}

#[test]
fn update_backend_operator_rotates_pubkey() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let new_operator = Pubkey::new_unique();
    let owner_pubkey = context.owner.pubkey();
    let instruction = update_backend_operator_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        new_operator,
    );
    let owner_keypair = context.owner.insecure_clone();
    context.send_signed(instruction, &[&owner_keypair]).unwrap();

    let group = context.deserialize_group(&group_pda);
    assert_eq!(group.backend_operator, new_operator);
}

#[test]
fn emergency_withdraw_sweeps_full_balance_and_rejects_non_owner() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);
    let (agent_pda, _, agent_token_account) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let funded_amount = 25_000_000;
    context.mint_to(&mint_authority, &mint, &agent_token_account, funded_amount);
    assert_eq!(context.token_balance(&agent_token_account), funded_amount);

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let destination_owner = Keypair::new();
    context.airdrop(&destination_owner.pubkey(), STARTING_LAMPORTS);
    let destination_token_account =
        context.create_ata(&destination_owner, &mint, &destination_owner.pubkey());

    let attacker = Keypair::new();
    context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);
    let attacker_instruction = emergency_withdraw_instruction(
        &context.program_id,
        &attacker.pubkey(),
        &group_pda,
        &agent_pda,
        &agent_token_account,
        &destination_token_account,
        0,
    );
    let attacker_result = context.send_signed(attacker_instruction, &[&attacker]);
    assert!(
        attacker_result.is_err(),
        "non-owner emergency_withdraw must fail"
    );

    let owner_instruction = emergency_withdraw_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &agent_token_account,
        &destination_token_account,
        0,
    );
    context
        .send_signed(owner_instruction, &[&owner_keypair])
        .unwrap();

    assert_eq!(context.token_balance(&agent_token_account), 0);
    assert_eq!(
        context.token_balance(&destination_token_account),
        funded_amount
    );
}

#[test]
fn emergency_withdraw_sweeps_non_bound_mint_accumulated_via_swaps() {
    // Provision an agent bound to mint A; plant a balance of a different mint
    // M (the kind of residual a swap could land in custody) and sweep it.
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let bound_mint = context.create_mint(&mint_authority, 6);
    let (agent_pda, _, _) = add_first_agent(&mut context, group_pda, bound_mint, None, None, None);

    // A non-bound mint M with a PDA-owned ATA holding tokens.
    let other_mint = context.create_mint(&mint_authority, 6);
    let owner_keypair = context.owner.insecure_clone();
    let agent_other_ata = context.create_ata(&owner_keypair, &other_mint, &agent_pda);
    let funded_amount = 7_500_000;
    context.mint_to(&mint_authority, &other_mint, &agent_other_ata, funded_amount);

    let destination_owner = Keypair::new();
    context.airdrop(&destination_owner.pubkey(), STARTING_LAMPORTS);
    let destination_other_ata =
        context.create_ata(&destination_owner, &other_mint, &destination_owner.pubkey());

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let instruction = emergency_withdraw_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &agent_other_ata,
        &destination_other_ata,
        0,
    );
    context.send_signed(instruction, &[&owner_keypair]).unwrap();
    assert_eq!(context.token_balance(&agent_other_ata), 0);
    assert_eq!(context.token_balance(&destination_other_ata), funded_amount);
}

#[test]
fn emergency_withdraw_rejects_mint_mismatch_between_agent_and_destination() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint_a = context.create_mint(&mint_authority, 6);
    let (agent_pda, _, agent_token_account) =
        add_first_agent(&mut context, group_pda, mint_a, None, None, None);
    context.mint_to(&mint_authority, &mint_a, &agent_token_account, 1_000_000);

    let mint_b = context.create_mint(&mint_authority, 6);
    let destination_owner = Keypair::new();
    context.airdrop(&destination_owner.pubkey(), STARTING_LAMPORTS);
    let destination_b_ata =
        context.create_ata(&destination_owner, &mint_b, &destination_owner.pubkey());

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let instruction = emergency_withdraw_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &agent_pda,
        &agent_token_account, // mint A
        &destination_b_ata,   // mint B
        0,
    );
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "mint mismatch must reject");
}

fn add_external_entry(
    context: &mut TestContext,
    group_pda: Pubkey,
    target: Pubkey,
    ttl_expires_at: i64,
    approved_amount: u64,
) -> Pubkey {
    let owner_pubkey = context.owner.pubkey();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::EXTERNAL,
        ttl_expires_at,
        approved_amount,
    );
    let owner_keypair = context.owner.insecure_clone();
    context.send_signed(instruction, &[&owner_keypair]).unwrap();
    entry_pda
}

#[test]
fn add_to_whitelist_external_happy_path() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, target, now + 86_400, 50_000_000);

    let entry = context.deserialize_whitelist(&entry_pda);
    assert_eq!(entry.entry_type, entry_type::EXTERNAL);
    assert_eq!(entry.ttl_expires_at, now + 86_400);
    assert_eq!(entry.approved_amount, 50_000_000);
    assert_eq!(entry.amount_used, 0);
}

#[test]
fn add_to_whitelist_external_rejects_past_ttl() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let owner_pubkey = context.owner.pubkey();
    let target = Pubkey::new_unique();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::EXTERNAL,
        now,
        50_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "past TTL must reject");
}

#[test]
fn add_to_whitelist_external_rejects_zero_amount() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let owner_pubkey = context.owner.pubkey();
    let target = Pubkey::new_unique();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::EXTERNAL,
        now + 86_400,
        0,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "zero approved_amount must reject");
}

#[test]
fn add_to_whitelist_protocol_forces_zero_ttl_and_amount() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let owner_pubkey = context.owner.pubkey();
    let target = Pubkey::new_unique();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::PROTOCOL,
        1_700_000_000,
        9_999_999,
    );
    let owner_keypair = context.owner.insecure_clone();
    context.send_signed(instruction, &[&owner_keypair]).unwrap();

    let entry = context.deserialize_whitelist(&entry_pda);
    assert_eq!(entry.entry_type, entry_type::PROTOCOL);
    assert_eq!(entry.ttl_expires_at, 0);
    assert_eq!(entry.approved_amount, 0);
}

#[test]
fn add_to_whitelist_rejects_non_owner_signer() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let attacker = Keypair::new();
    context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);

    let target = Pubkey::new_unique();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &attacker.pubkey(),
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::EXTERNAL,
        now + 86_400,
        50_000_000,
    );
    let result = context.send_signed(instruction, &[&attacker]);
    assert!(result.is_err(), "non-owner add_to_whitelist must fail");
}

#[test]
fn add_to_whitelist_rejects_intra_group_type() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let owner_pubkey = context.owner.pubkey();
    let target = Pubkey::new_unique();
    let (entry_pda, _) = context.whitelist_pda(&group_pda, &target);
    let instruction = add_to_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        MERCHANT_LABEL,
        entry_type::INTRA_GROUP,
        0,
        0,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "intra-group type via add_to_whitelist must fail");
}

#[test]
fn renew_whitelist_entry_happy_path_keeps_pda() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, target, now + 1_000, 10_000_000);

    let owner_pubkey = context.owner.pubkey();
    let instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        now + 86_400,
        20_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    context.send_signed(instruction, &[&owner_keypair]).unwrap();

    let entry = context.deserialize_whitelist(&entry_pda);
    assert_eq!(entry.ttl_expires_at, now + 86_400);
    assert_eq!(entry.approved_amount, 20_000_000);
    let (rederived_pda, _) = context.whitelist_pda(&group_pda, &target);
    assert_eq!(rederived_pda, entry_pda);
}

#[test]
fn renew_whitelist_entry_rejects_past_ttl() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, target, now + 1_000, 10_000_000);

    let owner_pubkey = context.owner.pubkey();
    let instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        now,
        20_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "renew with past TTL must fail");
}

#[test]
fn renew_whitelist_entry_rejects_lower_amount_than_used() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, target, now + 1_000, 10_000_000);

    // Forge amount_used directly to simulate prior consumption.
    let mut account = context.fetch_account(&entry_pda);
    let mut entry =
        enclz::state::WhitelistEntry::try_deserialize(&mut account.data.as_slice()).unwrap();
    entry.amount_used = 5_000_000;
    let mut data = Vec::with_capacity(account.data.len());
    entry.try_serialize(&mut data).unwrap();
    account.data = data;
    context.svm.set_account(entry_pda, account).unwrap();

    let owner_pubkey = context.owner.pubkey();
    let instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
        now + 86_400,
        4_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "renew with amount < used must fail");
}

#[test]
fn renew_whitelist_entry_rejects_intra_group_target() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);
    let (agent_pda, intra_entry, _) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let owner_pubkey = context.owner.pubkey();
    let instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &intra_entry,
        agent_pda,
        now + 86_400,
        20_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "renew on intra-group entry must fail");
}

#[test]
fn renew_whitelist_entry_rejects_protocol_target() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let (router_entry, _) = context.whitelist_pda(&group_pda, &dex_router);
    let owner_pubkey = context.owner.pubkey();
    let instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &router_entry,
        dex_router,
        now + 86_400,
        20_000_000,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "renew on protocol entry must fail");
}

#[test]
fn remove_from_whitelist_external_and_protocol_close_pda() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, target, now + 86_400, 10_000_000);

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();
    let remove_external = remove_from_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        target,
    );
    context.send_signed(remove_external, &[&owner_keypair]).unwrap();
    assert!(
        context.try_fetch_account(&entry_pda).is_none()
            || context.try_fetch_account(&entry_pda).unwrap().lamports == 0,
        "external entry should be closed"
    );

    let (router_entry, _) = context.whitelist_pda(&group_pda, &dex_router);
    let remove_protocol = remove_from_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &router_entry,
        dex_router,
    );
    context.send_signed(remove_protocol, &[&owner_keypair]).unwrap();
    assert!(
        context.try_fetch_account(&router_entry).is_none()
            || context.try_fetch_account(&router_entry).unwrap().lamports == 0,
        "protocol entry should be closed"
    );
}

#[test]
fn remove_from_whitelist_rejects_intra_group() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);
    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);
    let (agent_pda, intra_entry, _) =
        add_first_agent(&mut context, group_pda, mint, None, None, None);

    let owner_pubkey = context.owner.pubkey();
    let instruction = remove_from_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &intra_entry,
        agent_pda,
    );
    let owner_keypair = context.owner.insecure_clone();
    let result = context.send_signed(instruction, &[&owner_keypair]);
    assert!(result.is_err(), "intra-group remove must fail");
}

#[test]
fn full_provisioning_flow_init_two_agents_external_renew_remove() {
    let mut context = TestContext::new();
    let (backend_operator, protocol_fee_wallet, dex_router) = unique_keys();
    let group_pda =
        provision_group_with_router(&mut context, backend_operator, protocol_fee_wallet, dex_router);

    let mint_authority = Keypair::new();
    context.airdrop(&mint_authority.pubkey(), STARTING_LAMPORTS);
    let mint = context.create_mint(&mint_authority, 6);

    let owner_pubkey = context.owner.pubkey();
    let owner_keypair = context.owner.insecure_clone();

    add_first_agent(&mut context, group_pda, mint, None, None, None);

    let (second_agent_pda, _) = context.agent_pda(&group_pda, 1);
    let (second_intra, _) = context.whitelist_pda(&group_pda, &second_agent_pda);
    let second_token_account = context.associated_token_address(&second_agent_pda, &mint);
    let second_instruction = add_agent_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &second_agent_pda,
        &second_intra,
        &second_token_account,
        &mint,
        SECOND_NAME,
        Some(20_000_000),
        None,
        None,
    );
    context.send_signed(second_instruction, &[&owner_keypair]).unwrap();

    let group_after = context.deserialize_group(&group_pda);
    assert_eq!(group_after.agent_count, 2);

    let now = context.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp;
    let merchant = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut context, group_pda, merchant, now + 1_000, 10_000_000);

    let renew_instruction = renew_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        merchant,
        now + 86_400,
        20_000_000,
    );
    context.send_signed(renew_instruction, &[&owner_keypair]).unwrap();
    let renewed = context.deserialize_whitelist(&entry_pda);
    assert_eq!(renewed.approved_amount, 20_000_000);
    assert_eq!(renewed.ttl_expires_at, now + 86_400);

    let remove_instruction = remove_from_whitelist_instruction(
        &context.program_id,
        &owner_pubkey,
        &group_pda,
        &entry_pda,
        merchant,
    );
    context.send_signed(remove_instruction, &[&owner_keypair]).unwrap();
    let post_remove = context.try_fetch_account(&entry_pda);
    assert!(
        post_remove.is_none() || post_remove.unwrap().lamports == 0,
        "merchant entry should be closed"
    );
}
