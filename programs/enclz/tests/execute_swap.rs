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
        context.mint_to(
            &mint_authority,
            &mint,
            &agent_token_account,
            initial_balance,
        );
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
    );
    let owner_keypair = setup.context.owner.insecure_clone();
    setup
        .context
        .send_signed(instruction, &[&owner_keypair])
        .unwrap();
    entry_pda
}

fn add_external_entry(setup: &mut Setup, target: Pubkey, ttl_expires_at: i64) -> Pubkey {
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
    );
    let owner_keypair = setup.context.owner.insecure_clone();
    setup
        .context
        .send_signed(instruction, &[&owner_keypair])
        .unwrap();
    entry_pda
}

/// Output ATA owned by the agent_wallet PDA on a fresh mint — the realistic
/// "swap into a novel mint" shape. Returns the ATA pubkey.
fn pda_owned_output_ata(setup: &mut Setup, output_mint: &Pubkey) -> Pubkey {
    // litesvm_token's CreateAssociatedTokenAccount lets the payer create an
    // ATA whose authority is any pubkey — no signature from the authority is
    // needed since the SPL ATA program signs with the deterministic seed.
    let payer = setup.context.owner.insecure_clone();
    setup
        .context
        .create_ata(&payer, output_mint, &setup.agent_pda)
}

#[allow(clippy::too_many_arguments)]
fn execute_swap(
    setup: &mut Setup,
    operator: &Keypair,
    from_token_account: Pubkey,
    to_token_account: Pubkey,
    whitelist_entry: Pubkey,
    input_mint: Pubkey,
    protocol_fee_token_account: Pubkey,
    jupiter_program: Pubkey,
    amount_in: u64,
    expected_nonce: u64,
    route_data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> litesvm::types::TransactionResult {
    let protocol_fee_wallet = setup.protocol_fee_wallet.pubkey();
    let instruction = execute_swap_instruction(
        &setup.context.program_id,
        &operator.pubkey(),
        &setup.group_pda,
        &setup.agent_pda,
        &from_token_account,
        &to_token_account,
        &whitelist_entry,
        &input_mint,
        &protocol_fee_token_account,
        &protocol_fee_wallet,
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

    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    // Output ATA owned by the agent_wallet PDA — custody pin requirement.
    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
        stub_program_id,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(result.is_ok(), "swap should succeed: {result:?}");

    assert_eq!(
        setup
            .context
            .token_balance(&setup.protocol_fee_token_account),
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
    // spent_today is NOT touched on the swap path under the new policy —
    // daily/per-tx limits are mint-relative and don't apply across mints.
    assert_eq!(agent.spent_today, 0);
    assert_eq!(agent.tx_count_this_hour, 1);
}

// ---------------------------------------------------------------------------
// Negative paths
// ---------------------------------------------------------------------------

#[test]
fn non_type_2_whitelist_entry_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let now = setup
        .context
        .svm
        .get_sysvar::<solana_clock::Clock>()
        .unix_timestamp;
    let target = Pubkey::new_unique();
    let entry_pda = add_external_entry(&mut setup, target, now + 86_400);

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    // The whitelist seed binds to `target`; pass the same pubkey as the
    // jupiter_program so seed validation passes — the entry_type check is
    // what should fail.
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        entry_pda,
        bound_mint,
        fee_ata,
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

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
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
fn hourly_cap_reached_rejects() {
    let mut setup = setup_with_funded_agent(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let now = setup
        .context
        .svm
        .get_sysvar::<solana_clock::Clock>()
        .unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.tx_count_this_hour = agent.hourly_tx_cap;
        agent.last_hour_reset = now;
    });

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
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

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let attacker = Keypair::new();
    setup.context.airdrop(&attacker.pubkey(), STARTING_LAMPORTS);
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &attacker,
        from_ata,
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
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

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    // ATA owned by a non-agent keypair.
    let bystander = Keypair::new();
    setup
        .context
        .airdrop(&bystander.pubkey(), STARTING_LAMPORTS);
    let bystander_ata = setup
        .context
        .create_ata(&bystander, &setup.mint, &bystander.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &operator,
        bystander_ata, // wrong from
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
        stub_program_id,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::InvalidTokenAccount);
}

#[test]
fn to_token_account_third_party_owner_rejects() {
    let mut setup = setup_with_funded_agent(5_000_000);
    let stub_program_id = Pubkey::new_unique();
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    // Output ATA owned by an unrelated keypair — custody pin must reject.
    let third_party = Keypair::new();
    setup
        .context
        .airdrop(&third_party.pubkey(), STARTING_LAMPORTS);
    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let third_party_ata =
        setup
            .context
            .create_ata(&third_party, &output_mint, &third_party.pubkey());

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        third_party_ata,
        router_entry,
        bound_mint,
        fee_ata,
        stub_program_id,
        500_000,
        0,
        vec![0u8],
        vec![],
    );
    assert_anchor_error(result, EnclzError::InvalidTokenAccount);
}

#[test]
fn swap_allows_arbitrary_input_mint_into_pda_owned_output() {
    // Agent is bound to mint A (the setup mint). We swap from a SECOND mint B
    // (held in a PDA-owned ATA) into an OUTPUT ATA on a third mint C, also
    // PDA-owned. The instruction must succeed even though neither input nor
    // output mint equals `agent_wallet.mint`.
    let mut setup = setup_with_funded_agent(0);
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let mint_b = setup.context.create_mint(&setup.mint_authority, 6);
    let mint_c = setup.context.create_mint(&setup.mint_authority, 6);

    let agent_pda = setup.agent_pda;
    let from_ata_b = setup.context.associated_token_address(&agent_pda, &mint_b);
    // create the agent ATA for mint B
    let payer = setup.context.owner.insecure_clone();
    setup.context.create_ata(&payer, &mint_b, &agent_pda);
    setup
        .context
        .mint_to(&setup.mint_authority, &mint_b, &from_ata_b, 5_000_000);

    // PDA-owned output ATA on mint C
    let to_ata_c = pda_owned_output_ata(&mut setup, &mint_c);

    // Pre-create the protocol_fee ATA for mint B so init_if_needed sees it.
    let fee_ata_b = setup.context.create_ata(
        &setup.protocol_fee_wallet,
        &mint_b,
        &setup.protocol_fee_wallet.pubkey(),
    );

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata_b,
        to_ata_c,
        router_entry,
        mint_b,
        fee_ata_b,
        stub_program_id,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(
        result.is_ok(),
        "swap with non-bound mints should succeed: {result:?}"
    );

    // spent_today must remain unchanged — swap path does not touch it.
    let agent = setup.context.deserialize_agent(&agent_pda);
    assert_eq!(agent.spent_today, 0);
    assert_eq!(agent.tx_count_this_hour, 1);
}

#[test]
fn swap_does_not_enforce_per_tx_or_daily_limit() {
    // Set spent_today and a tiny per_tx limit; swap should still succeed.
    let mut setup = setup_with_funded_agent(50_000_000);
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let now = setup
        .context
        .svm
        .get_sysvar::<solana_clock::Clock>()
        .unix_timestamp;
    let agent_pda = setup.agent_pda;
    setup.context.rewrite_agent(&agent_pda, |agent| {
        agent.spent_today = 9_500_000; // close to default daily_limit (10_000_000)
        agent.last_spend_reset = now;
        agent.per_tx_limit = 1_000; // tiny
    });

    let output_mint = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata = pda_owned_output_ata(&mut setup, &output_mint);

    let operator = setup.backend_operator.insecure_clone();
    let from_ata = setup.agent_token_account;
    let fee_ata = setup.protocol_fee_token_account;
    let bound_mint = setup.mint;
    // amount_in (5_000_000) is far above per_tx_limit and would push past
    // daily_limit if it were enforced. New policy: only hourly_tx_cap gates.
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata,
        to_ata,
        router_entry,
        bound_mint,
        fee_ata,
        stub_program_id,
        5_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(
        result.is_ok(),
        "swap should bypass per_tx and daily limits: {result:?}"
    );
    let agent = setup.context.deserialize_agent(&agent_pda);
    // Pre-existing spent_today is untouched by the swap.
    assert_eq!(agent.spent_today, 9_500_000);
}

#[test]
fn lazy_init_fee_ata_for_novel_mint() {
    // First swap of a novel input mint must auto-create the fee ATA via
    // init_if_needed and charge rent to backend_operator.
    let mut setup = setup_with_funded_agent(0);
    let stub_program_id = Pubkey::new_unique();
    setup.context.add_stub_program(&stub_program_id);
    let router_entry = add_protocol_entry(&mut setup, stub_program_id, PROTOCOL_LABEL);

    let mint_b = setup.context.create_mint(&setup.mint_authority, 6);
    let agent_pda = setup.agent_pda;
    let from_ata_b = setup.context.associated_token_address(&agent_pda, &mint_b);
    let payer = setup.context.owner.insecure_clone();
    setup.context.create_ata(&payer, &mint_b, &agent_pda);
    setup
        .context
        .mint_to(&setup.mint_authority, &mint_b, &from_ata_b, 5_000_000);

    let mint_c = setup.context.create_mint(&setup.mint_authority, 6);
    let to_ata_c = pda_owned_output_ata(&mut setup, &mint_c);

    let fee_ata_b_addr = setup
        .context
        .associated_token_address(&setup.protocol_fee_wallet.pubkey(), &mint_b);
    // Confirm the fee ATA does NOT yet exist for mint B.
    assert!(
        setup.context.try_fetch_account(&fee_ata_b_addr).is_none(),
        "fee ATA for mint B must not exist before swap"
    );

    let pre_operator_lamports = setup
        .context
        .fetch_account(&setup.backend_operator.pubkey())
        .lamports;

    let operator = setup.backend_operator.insecure_clone();
    let result = execute_swap(
        &mut setup,
        &operator,
        from_ata_b,
        to_ata_c,
        router_entry,
        mint_b,
        fee_ata_b_addr,
        stub_program_id,
        1_000_000,
        0,
        vec![0u8],
        vec![],
    );
    assert!(result.is_ok(), "lazy-init swap should succeed: {result:?}");

    // Fee ATA must now exist and hold the 1_000 protocol fee.
    let fee_account = setup
        .context
        .try_fetch_account(&fee_ata_b_addr)
        .expect("fee ATA should be initialized");
    assert!(fee_account.lamports > 0, "fee ATA should be rent-exempt");
    assert_eq!(setup.context.token_balance(&fee_ata_b_addr), 1_000);

    let post_operator_lamports = setup
        .context
        .fetch_account(&setup.backend_operator.pubkey())
        .lamports;
    assert!(
        post_operator_lamports < pre_operator_lamports,
        "operator should have paid rent for the fee ATA"
    );
}

#[test]
fn fee_plus_net_property_pinned_by_lib_unit_test() {
    // Sanity-check the boundary cases here too so the property is documented
    // alongside the swap-instruction tests. With additive fee, total = amount +
    // ceil(amount * 10 / 10000), so total - fee = amount.
    use enclz::util::fee::compute_fee;
    for &amount in &[1u64, 100, 999, 1_000, 1_000_000, 10_050_000, u64::MAX / 2] {
        let (total, fee) = compute_fee(amount).unwrap();
        assert_eq!(total.checked_sub(fee), Some(amount), "amount = {amount}");
    }
}
