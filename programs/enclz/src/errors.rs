use anchor_lang::prelude::*;

#[error_code]
pub enum EnclzError {
    #[msg("Recipient address is not whitelisted for this group")]
    WhitelistViolation,
    #[msg("Whitelist entry has expired")]
    WhitelistExpired,
    #[msg("Whitelist entry approved amount has been exhausted")]
    /// Retired — per-recipient spending cap removed. Never emitted.
    WhitelistAmountExhausted,
    #[msg("Daily spend limit exceeded")]
    DailyLimitExceeded,
    #[msg("Per-transaction limit exceeded")]
    PerTxLimitExceeded,
    #[msg("Hourly transaction cap exceeded")]
    HourlyCapExceeded,
    #[msg("Operator nonce mismatch — possible replay")]
    NonceMismatch,
    #[msg("Caller is not authorized to perform this action")]
    Unauthorized,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("TTL must be in the future")]
    InvalidTtl,
    #[msg("Group has reached its maximum agent count")]
    TooManyAgents,
    #[msg("Token account mint does not match across transfer legs")]
    InvalidMint,
    #[msg("Protocol fee token account does not match group_config.protocol_fee_wallet")]
    InvalidFeeAccount,
    #[msg("Token account owner does not match the expected agent_wallet PDA")]
    InvalidTokenAccount,
    #[msg("Recipient is the protocol fee wallet or the agent PDA")]
    RecipientInvalid,
    #[msg("Whitelist entry type is not recognized")]
    InvalidEntryType,
}
