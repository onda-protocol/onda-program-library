use anchor_lang::{
  prelude::*,
  solana_program::{
      program::{invoke},
  }
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::utils::*;
use crate::constants::*;
use crate::error::*;
use crate::state::{Loan, LoanState, TokenManager};
use crate::processor::loan::close::{process_close_loan};

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub lender: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        has_one = borrower,
        has_one = mint,
        constraint = loan.lender.unwrap() == lender.key(), 
        constraint = loan.state == LoanState::Active,
        close = borrower,
    )]
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: deserialized and validated
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_repay_loan<'info>(ctx: Context<'_, '_, '_, 'info, RepayLoan<'info>>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let borrower = &ctx.accounts.borrower;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let token_manager = &mut ctx.accounts.token_manager;
    let mint = &ctx.accounts.mint;
    let edition = &ctx.accounts.edition;

    let duration = ctx.accounts.clock.unix_timestamp.checked_sub(
        loan.start_date.unwrap()
    ).ok_or(ErrorCodes::NumericalOverflow)?;

    let expiry = loan.start_date.unwrap().checked_add(loan.duration).ok_or(ErrorCodes::NumericalOverflow)?;
    let is_overdue = ctx.accounts.clock.unix_timestamp > expiry;
    
    let interest_due = calculate_loan_repayment_fee(
        loan.amount.unwrap(),
        loan.basis_points,
        duration,
        is_overdue
    )?;
    let amount_due = loan.amount.unwrap().checked_add(interest_due).ok_or(ErrorCodes::NumericalOverflow)?;

    invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &loan.borrower,
            &loan.lender.unwrap(),
            amount_due,
        ),
        &[
            borrower.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ]
    )?;

    let creator_fee = calculate_loan_repayment_fee(
        loan.amount.unwrap(),
        loan.creator_basis_points,
        duration,
        false
    )?;

    pay_creator_fees(
        creator_fee,
        10_000, // 100%
        &mint.to_account_info(),
        &ctx.accounts.metadata.to_account_info(),
        &mut borrower.to_account_info(),
        &mut ctx.remaining_accounts.iter(),
    )?;

    process_close_loan(
        token_manager,
        borrower,
        deposit_token_account,
        mint,
        edition,
        &ctx.accounts.token_program
    )?;

    Ok(())
}