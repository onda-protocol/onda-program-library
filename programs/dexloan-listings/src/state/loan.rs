use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum LoanState {
    Unlisted,
    Listed,
    Active,
    Defaulted,
}

#[account]
pub struct Loan {
    /// Whether the loan is active
    pub state: LoanState,
    /// The amount of the loan
    pub amount: Option<u64>,
    /// The amount outstanding
    pub outstanding: u64,
    /// The liquidation threshold in basis points
    pub threshold: Option<u32>,
    /// The NFT holder
    pub borrower: Pubkey,
    /// The issuer of the loan
    pub lender: Option<Pubkey>,
    /// Annual percentage yield
    pub basis_points: u32,
    /// Number of installments
    pub installments: u8,
    /// Current installment
    pub current_installment: u8,
    /// Notice issued ts
    pub notice_issued: Option<i64>,
    /// Duration of the loan in seconds
    pub duration: i64,
    /// The start date of the loan
    pub start_date: Option<i64>,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// The mint of the spl-token mint
    pub token_mint: Option<Pubkey>,
    /// misc
    pub bump: u8,
}

impl Loan {
    pub fn space() -> usize {
        8 + // key
        1 + // state
        (1 + 8) + // amount
        8 + // outstanding
        (1 + 4) + // threshold
        32 + // borrower
        (1 + 32) + // lender
        4 + // basis_points
        1 + // installments
        1 + // current_installment
        (1 + 8) + // notice_issued
        8 + // duration
        (1 + 8) + // start_date
        32 + // mint
        (1 + 32) + // token_mint
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"loan";
}