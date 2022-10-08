use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::*;

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
    pub fn init_state(
        mut self,
        amount: u64,
        basis_points: u32,
        duration: i64
    ) {
        self.amount = Some(amount);
        self.outstanding = amount;
        self.threshold = None;
        self.installments = 1;
        self.basis_points = basis_points;
        self.duration = duration;
        self.state = LoanState::Listed;
    }   

    pub fn set_active(mut self, unix_timestamp: i64) {
        require_eq!(self.state, LoanState::Listed, DexloanError::InvalidExpiry);
        require!(self.amount.is_some(), DexloanError::InvalidState);
        require!(self.lender.is_some(), DexloanError::InvalidState);
        require!(self.borrower != SYSTEM_ACCOUNT, DexloanError::InvalidState);
        require!(self.outstanding == self.amount.unwrap(), DexloanError::InvalidState);
        require!(self.installments > 0, DexloanError::InvalidState);
        require!(self.basis_points >= 0, DexloanError::InvalidState);
        require!(self.duration > 0, DexloanError::InvalidState);

        self.state = LoanState::Active;
        self.start_date = Some(unix_timestamp);

        require!(self.start_date.is_some(), DexloanError::InvalidState);
    }

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


#[account]
pub struct LoanOffer {
    /// id of the offer
    pub id: u8, 
    /// The lender making the offer
    pub lender: Pubkey,
    /// The amount of the loan
    pub amount: Option<u64>,
    /// Annual percentage yield
    pub basis_points: u32,
    /// Duration of the loan in seconds
    pub duration: i64,
    /// The collection
    pub collection: Pubkey,
    /// The loan to floor-value of the offer
    pub ltv: Option<u32>,
    /// The liquidation threshold in basis points
    pub threshold: Option<u32>,
    /// misc
    pub bump: u8,
    pub escrow_bump: u8,
}

impl LoanOffer {
    pub fn space() -> usize {
        8 + // key
        1 + // id
        32 + // lender
        (1 + 8) + // amount
        4 + // basis_points
        8 + // duration
        32 + // collection
        (1 + 4) + // ltv
        (1 + 4) + // threshold
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"loan_offer";
    pub const VAULT_PREFIX: &'static [u8] = b"loan_offer_vault";
}