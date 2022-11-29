use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum RentalState {
    Listed,
    Rented,
}

#[account]
pub struct Rental {
    /// Whether the loan is active
    pub state: RentalState,
    /// The daily cost to rental
    pub amount: u64,
    /// The creator fee
    pub creator_basis_points: u16,
    /// The NFT lender
    pub lender: Pubkey,
    /// The NFT borrower
    pub borrower: Option<Pubkey>,
    /// The latest date this NFT may be rentald until
    pub expiry: i64,
    /// The start date of the current rental
    pub current_start: Option<i64>,
    /// The end date of the current rental
    pub current_expiry: Option<i64>,
    /// Any amount withheld in escrow
    pub escrow_balance: u64,
    /// The mint of the token being used for collateral,
    pub mint: Pubkey,
    /// Misc
    pub bump: u8,
}

impl Rental {
    pub fn space() -> usize {
        8 + // key
        1 + // state
        8 + // amount
        2 + // creator_basis_points
        32 + // lender
        (1 + 32) + // borrower
        8 + // expiry
        (1 + 8) + // current_start
        (1 + 8) + // current_expiry
        8 + // escrow_balance
        32 + // mint
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"rental";
    pub const ESCROW_PREFIX: &'static [u8] = b"rental_escrow";
}