use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum LoanState {
    Unlisted,
    Listed,
    Active,
    Defaulted,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum InterestType {
    Fixed,
    Variable,
}

#[account]
pub struct Loan {
    /// Whether the loan is active
    pub state: LoanState,
    /// Loan to floor value
    pub ltv: Option<u32>,
    /// The amount of the loan
    pub amount: Option<u64>,
    /// The liquidation threshold
    pub threshold: Option<u32>,
    /// The NFT holder
    pub borrower: Pubkey,
    /// The issuer of the loan
    pub lender: Option<Pubkey>,
    /// Annualized return
    pub basis_points: u32,
    /// Interest rate type
    pub interest_rate_type: InterestType,
    /// Early repayment fee in basis points
    pub early_repayment_fee: u32,
    /// Duration of the loan in seconds
    pub duration: i64,
    /// The start date of the loan
    pub start_date: Option<i64>,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// (Optional) The mint of the spl-token mint
    pub token_mint: Option<Pubkey>,
    /// misc
    pub bump: u8,
}

impl Loan {
    pub fn space() -> usize {
        8 + // key
        1 + // state
        (1 + 4) + // Loan to value
        (1 + 8) + // amount
        (1 + 4) + // threshold
        (1 + 8) + // Sale price
        32 + // borrower
        (1 + 32) + // lender
        4 + // basis_points
        8 + // duration
        (1 + 8) + // start_date
        32 + // mint
        (1 + 32) + // padding
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"loan";
}