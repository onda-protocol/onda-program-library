use anchor_lang::{prelude::*};
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::{Loan, LoanState, Hire, HireState, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(Accounts)]
pub struct RepossessWithHire<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    /// CHECK: contrained on hire_account
    #[account(mut)]
    pub hire_borrower: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub lender_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        has_one = borrower,
        has_one = lender,
        has_one = mint,
        constraint = loan.state == LoanState::Active,
    )]
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        mut,
        seeds = [
            Hire::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
        constraint = hire.state == HireState::Hired,
        constraint = hire.borrower.is_some() && hire.borrower.unwrap() == hire_borrower.key(), 
    )]
    pub hire: Box<Account<'info, Hire>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = hire_borrower
    )]
    pub hire_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        bump,
        constraint = token_manager.accounts.hire == true,
        constraint = token_manager.accounts.loan == true,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    /// CHECK: contrained on loan_account
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_repossess_with_hire(ctx: Context<RepossessWithHire>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let hire = &mut ctx.accounts.hire;
    let token_manager = &mut ctx.accounts.token_manager;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp;
    let start_date = loan.start_date;
    let duration = unix_timestamp - start_date;

    if loan.duration > duration  {
        return Err(DexloanError::NotOverdue.into())
    }

    loan.state = LoanState::Defaulted;
    token_manager.accounts.loan = false;
    token_manager.accounts.hire = false;

    settle_hire_escrow_balance(
        hire,
        ctx.accounts.hire_borrower.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        unix_timestamp,
    )?;

    thaw_and_transfer_from_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.lender.to_account_info(),
        ctx.accounts.hire_token_account.to_account_info(),
        ctx.accounts.lender_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
    )?;

    Ok(())
}