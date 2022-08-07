use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Loan, LoanState};

#[derive(Accounts)]
pub struct GiveLoan<'info> {
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.borrower != lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed,
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}


pub fn handle_give_loan(ctx: Context<GiveLoan>) -> Result<()> {
  let listing = &mut ctx.accounts.loan_account;

  listing.state = LoanState::Active;
  listing.lender = ctx.accounts.lender.key();
  listing.start_date = ctx.accounts.clock.unix_timestamp;
  // Transfer amount
  anchor_lang::solana_program::program::invoke(
      &anchor_lang::solana_program::system_instruction::transfer(
          &listing.lender,
          &listing.borrower,
          listing.amount,
      ),
      &[
          ctx.accounts.lender.to_account_info(),
          ctx.accounts.borrower.to_account_info(),
      ]
  )?;

  Ok(())
}