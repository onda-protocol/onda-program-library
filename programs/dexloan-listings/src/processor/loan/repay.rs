use anchor_lang::{
  prelude::*,
  solana_program::{
      program::{invoke},
  }
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState};
use crate::utils::*;

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
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
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
  let loan = &mut ctx.accounts.loan_account;

  let amount_due = calculate_loan_repayment(
      loan.amount,
      loan.basis_points,
      loan.duration
  )?;

  // Transfer payment
  invoke(
      &anchor_lang::solana_program::system_instruction::transfer(
          &loan.borrower,
          &loan.lender,
          amount_due,
      ),
      &[
          ctx.accounts.borrower.to_account_info(),
          ctx.accounts.lender.to_account_info(),
      ]
  )?;

  let signer_bump = &[loan.bump];
  let signer_seeds = &[&[
      Loan::PREFIX,
      loan.mint.as_ref(),
      loan.borrower.as_ref(),
      signer_bump
  ][..]];

  thaw(
      FreezeParams {
          delegate: loan.to_account_info(),
          token_account: ctx.accounts.deposit_token_account.to_account_info(),
          edition: ctx.accounts.edition.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          signer_seeds: signer_seeds
      }
  )?;

  anchor_spl::token::revoke(
      CpiContext::new(
          ctx.accounts.token_program.to_account_info(),
          anchor_spl::token::Revoke {
              source: ctx.accounts.deposit_token_account.to_account_info(),
              authority: ctx.accounts.borrower.to_account_info(),
          }
      )
  )?;

  Ok(())
}