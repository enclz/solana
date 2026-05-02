use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;

pub use constants::*;
pub use errors::*;
pub use state::*;

declare_id!("Enc1zProgram11111111111111111111111111111111");

#[program]
pub mod enclz {
    #[allow(unused_imports)]
    use super::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn program_id() -> Pubkey {
        crate::ID
    }

    #[test]
    fn group_config_pda_matches_documented_seeds() {
        let owner = Pubkey::new_unique();
        let (expected, _bump) =
            Pubkey::find_program_address(&[GROUP_SEED, owner.as_ref()], &program_id());
        let (actual, _) =
            Pubkey::find_program_address(&[b"group", owner.as_ref()], &program_id());
        assert_eq!(expected, actual);
    }

    #[test]
    fn agent_wallet_pda_matches_documented_seeds() {
        let group = Pubkey::new_unique();
        let idx: u8 = 3;
        let (expected, _) = Pubkey::find_program_address(
            &[WALLET_SEED, group.as_ref(), &[idx]],
            &program_id(),
        );
        let (actual, _) = Pubkey::find_program_address(
            &[b"wallet", group.as_ref(), &[idx]],
            &program_id(),
        );
        assert_eq!(expected, actual);
    }

    #[test]
    fn whitelist_entry_pda_matches_documented_seeds() {
        let group = Pubkey::new_unique();
        let target = Pubkey::new_unique();
        let (expected, _) = Pubkey::find_program_address(
            &[WHITELIST_SEED, group.as_ref(), target.as_ref()],
            &program_id(),
        );
        let (actual, _) = Pubkey::find_program_address(
            &[b"whitelist", group.as_ref(), target.as_ref()],
            &program_id(),
        );
        assert_eq!(expected, actual);
    }

    #[test]
    fn init_space_group_config_matches_field_layout() {
        // 32 (owner) + 32 (backend_operator) + 32 (protocol_fee_wallet) + 1 (agent_count)
        assert_eq!(GroupConfig::INIT_SPACE, 32 + 32 + 32 + 1);
    }

    #[test]
    fn init_space_agent_wallet_matches_field_layout() {
        // 32 (group) + 32 (display_name) + 8 + 8 + 1 + 8 + 1 + 8 + 8 + 8 + 1 (bump)
        let expected = 32 + 32 + 8 + 8 + 1 + 8 + 1 + 8 + 8 + 8 + 1;
        assert_eq!(AgentWallet::INIT_SPACE, expected);
    }

    #[test]
    fn init_space_whitelist_entry_matches_field_layout() {
        // 32 (label) + 32 (added_by) + 1 (entry_type) + 8 (ttl) + 8 (approved) + 8 (used) + 1 (bump)
        let expected = 32 + 32 + 1 + 8 + 8 + 8 + 1;
        assert_eq!(WhitelistEntry::INIT_SPACE, expected);
    }

    #[test]
    fn group_config_round_trip_through_init_space_buffer() {
        let mut buf = vec![0u8; 8 + GroupConfig::INIT_SPACE];
        let mut cursor: &mut [u8] = &mut buf[8..];
        let value = GroupConfig {
            owner: Pubkey::new_unique(),
            backend_operator: Pubkey::new_unique(),
            protocol_fee_wallet: Pubkey::new_unique(),
            agent_count: 7,
        };
        AnchorSerialize::serialize(&value, &mut cursor).expect("serialize must fit");
        let decoded: GroupConfig =
            AnchorDeserialize::deserialize(&mut &buf[8..8 + GroupConfig::INIT_SPACE])
                .expect("decode must succeed");
        assert_eq!(decoded.owner, value.owner);
        assert_eq!(decoded.backend_operator, value.backend_operator);
        assert_eq!(decoded.protocol_fee_wallet, value.protocol_fee_wallet);
        assert_eq!(decoded.agent_count, value.agent_count);
    }

    #[test]
    fn agent_wallet_round_trip_through_init_space_buffer() {
        let mut buf = vec![0u8; 8 + AgentWallet::INIT_SPACE];
        let mut cursor: &mut [u8] = &mut buf[8..];
        let value = AgentWallet {
            group: Pubkey::new_unique(),
            display_name: [0xAB; 32],
            daily_limit: DEFAULT_DAILY_LIMIT,
            per_tx_limit: DEFAULT_PER_TX_LIMIT,
            hourly_tx_cap: DEFAULT_HOURLY_CAP,
            spent_today: 0,
            tx_count_this_hour: 0,
            last_spend_reset: 0,
            last_hour_reset: 0,
            operator_nonce: 0,
            bump: 254,
        };
        AnchorSerialize::serialize(&value, &mut cursor).expect("serialize must fit");
        let decoded: AgentWallet =
            AnchorDeserialize::deserialize(&mut &buf[8..8 + AgentWallet::INIT_SPACE])
                .expect("decode must succeed");
        assert_eq!(decoded.group, value.group);
        assert_eq!(decoded.daily_limit, DEFAULT_DAILY_LIMIT);
        assert_eq!(decoded.per_tx_limit, DEFAULT_PER_TX_LIMIT);
        assert_eq!(decoded.hourly_tx_cap, DEFAULT_HOURLY_CAP);
        assert_eq!(decoded.bump, 254);
    }

    #[test]
    fn whitelist_entry_round_trip_through_init_space_buffer() {
        let mut buf = vec![0u8; 8 + WhitelistEntry::INIT_SPACE];
        let mut cursor: &mut [u8] = &mut buf[8..];
        let value = WhitelistEntry {
            label: [0xCD; 32],
            added_by: Pubkey::new_unique(),
            entry_type: state::whitelist_entry::entry_type::EXTERNAL,
            ttl_expires_at: 1_700_000_000,
            approved_amount: 5_000_000,
            amount_used: 0,
            bump: 253,
        };
        AnchorSerialize::serialize(&value, &mut cursor).expect("serialize must fit");
        let decoded: WhitelistEntry =
            AnchorDeserialize::deserialize(&mut &buf[8..8 + WhitelistEntry::INIT_SPACE])
                .expect("decode must succeed");
        assert_eq!(decoded.entry_type, 1);
        assert_eq!(decoded.approved_amount, 5_000_000);
        assert_eq!(decoded.bump, 253);
    }

    #[test]
    fn entry_type_constants_match_spec() {
        assert_eq!(state::whitelist_entry::entry_type::INTRA_GROUP, 0);
        assert_eq!(state::whitelist_entry::entry_type::EXTERNAL, 1);
        assert_eq!(state::whitelist_entry::entry_type::PROTOCOL, 2);
    }

    #[test]
    fn seed_constants_match_documented_bytes() {
        assert_eq!(GROUP_SEED, b"group");
        assert_eq!(WALLET_SEED, b"wallet");
        assert_eq!(WHITELIST_SEED, b"whitelist");
    }

    #[test]
    fn default_limit_constants_match_spec() {
        assert_eq!(DEFAULT_DAILY_LIMIT, 10_000_000);
        assert_eq!(DEFAULT_PER_TX_LIMIT, 1_000_000);
        assert_eq!(DEFAULT_HOURLY_CAP, 5);
        assert_eq!(PROTOCOL_FEE_BPS, 10);
    }

    #[test]
    fn error_variants_have_stable_codes() {
        // Anchor assigns variant 0 → 6000, increments by 1.
        // Locking the offsets means backend pass-through stays stable.
        assert_eq!(EnclzError::WhitelistViolation as u32, 0);
        assert_eq!(EnclzError::WhitelistExpired as u32, 1);
        assert_eq!(EnclzError::WhitelistAmountExhausted as u32, 2);
        assert_eq!(EnclzError::DailyLimitExceeded as u32, 3);
        assert_eq!(EnclzError::PerTxLimitExceeded as u32, 4);
        assert_eq!(EnclzError::HourlyCapExceeded as u32, 5);
        assert_eq!(EnclzError::NonceMismatch as u32, 6);
        assert_eq!(EnclzError::Unauthorized as u32, 7);
        assert_eq!(EnclzError::InvalidAmount as u32, 8);
        assert_eq!(EnclzError::InvalidTtl as u32, 9);
    }
}
