use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum HireState {
    Listed,
    Hired,
}

#[account]
pub struct Hire {
    /// Whether the loan is active
    pub state: HireState,
    /// The daily cost to hire
    pub amount: u64,
    /// The NFT lender
    pub lender: Pubkey,
    /// The NFT borrower
    pub borrower: Option<Pubkey>,
    /// The expiry of the hire
    pub expiry: i64,
    /// The duration of the current hire
    pub current_expiry: Option<i64>,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// Misc
    pub bump: u8,
}

impl Hire {
    pub fn space() -> usize {
        8 + // key
        1 + // state
        8 + // amount
        32 + // lender
        (1 + 32) + // borrower
        8 + // expiry
        (1 + 8) + // current_expiry
        32 + // mint
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"hire";
}