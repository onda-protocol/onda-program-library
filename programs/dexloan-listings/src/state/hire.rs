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
    /// The cost to hire
    pub amount: u64,
    /// The NFT holder
    pub borrower: Pubkey,
    /// The issuer of the loan
    pub lender: Pubkey,
    /// The expiry of the hire
    pub expiry: i64,
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
        32 + // borrower
        32 + // lender
        8 + // expiry
        32 + // mint
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"hire";
}