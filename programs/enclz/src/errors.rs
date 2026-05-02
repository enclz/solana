use anchor_lang::prelude::*;

#[error_code]
pub enum EnclzError {
    #[msg("Recipient address is not whitelisted for this group")]
    WhitelistViolation,
    #[msg("Whitelist entry has expired")]
    WhitelistExpired,
    #[msg("Whitelist entry approved amount has been exhausted")]
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
}
