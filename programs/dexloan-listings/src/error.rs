use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("This loan is not overdue")]
    NotOverdue,
    #[msg("Invalid expiry")]
    InvalidExpiry,
    #[msg("Invalid state")]
    InvalidState,
    #[msg("Invalid listing type")]
    InvalidListingType,
    #[msg("Option expired")]
    OptionExpired,
    #[msg("Option not expired")]
    OptionNotExpired,
}