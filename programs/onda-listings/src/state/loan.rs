use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::*;

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Debug)]
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
    /// The NFT holder
    pub borrower: Pubkey,
    /// The issuer of the loan
    pub lender: Option<Pubkey>,
    /// The amount of the loan
    pub amount: Option<u64>,
    /// Annual percentage yield
    pub basis_points: u16,
    /// The creator fee
    pub creator_basis_points: u16,
    /// The amount outstanding
    pub outstanding: u64,
    /// The liquidation threshold in basis points
    pub threshold: Option<u32>,
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
    pub fn init_ask_state<'info>(
        loan: &mut Account<'info, Loan>,
        amount: u64,
        creator_basis_points: u16,
        basis_points: u16,
        duration: i64
    ) -> Result<()> {
        loan.amount = Some(amount);
        loan.basis_points = basis_points;
        loan.creator_basis_points = creator_basis_points;
        loan.outstanding = amount;
        loan.threshold = None;
        loan.installments = 1;
        loan.duration = duration;
        loan.state = LoanState::Listed;
    
        Ok(())
    }   
    
    pub fn set_active<'info>(loan: &mut Account<'info, Loan>, unix_timestamp: i64) -> Result<()> {
        if loan.state != LoanState::Listed {
            return err!(ErrorCodes::InvalidState);
        }
    
        require!(loan.lender.is_some(), ErrorCodes::InvalidState);
        require!(loan.amount.is_some(), ErrorCodes::InvalidState);
        require_keys_neq!(loan.borrower, SYSTEM_ACCOUNT, ErrorCodes::InvalidState);
        require_eq!(loan.outstanding, loan.amount.unwrap(), ErrorCodes::InvalidState);
        require_gt!(loan.installments, 0, ErrorCodes::InvalidState);
        require_gt!(loan.duration, 0, ErrorCodes::InvalidState);
        require_gte!(loan.basis_points, 0, ErrorCodes::InvalidState);
    
        loan.state = LoanState::Active;
        loan.start_date = Some(unix_timestamp);
    
        require!(loan.start_date.is_some(), ErrorCodes::InvalidState);
    
        Ok(())
    }

    pub fn space() -> usize {
        8 + // key
        1 + // state
        32 + // borrower
        (1 + 32) + // lender
        (1 + 8) + // amount
        2 + // basis_points
        2 + // creator_basis_points
        8 + // outstanding
        (1 + 4) + // threshold
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
    pub basis_points: u16,
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
        2 + // basis_points
        8 + // duration
        32 + // collection
        (1 + 4) + // ltv
        (1 + 4) + // threshold
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"loan_offer";
    pub const VAULT_PREFIX: &'static [u8] = b"loan_offer_vault";
}