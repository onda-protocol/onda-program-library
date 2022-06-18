use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use crate::state::{Listing, ListingState, Loan, LoanState};
use crate::error::{ErrorCode};

pub fn close(ctx: Context<CloseListing>) -> Result<()> {
  let listing = &mut ctx.accounts.loan_account;

  listing.close(ctx.accounts.borrower.to_account_info())?;

  Ok(())
}

#[derive(Accounts)]
pub struct CloseListing<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.state != LoanState::Listed,
        constraint = loan_account.state != LoanState::Active,
    )]
    pub loan_account: Account<'info, Loan>,
}