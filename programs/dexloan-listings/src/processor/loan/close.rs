use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState};
use crate::utils::*;

#[derive(Accounts)]
pub struct CloseLoan<'info> {
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = loan_account.bump,
        constraint = loan_account.borrower == *borrower.key,
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed || loan_account.state == LoanState::Defaulted,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_close_loan(ctx: Context<CloseLoan>) -> Result<()> {
  if ctx.accounts.deposit_token_account.is_frozen() {
      let signer_bump = &[ctx.accounts.loan_account.bump];
      let signer_seeds = &[&[
          Loan::PREFIX,
          ctx.accounts.loan_account.mint.as_ref(),
          ctx.accounts.loan_account.borrower.as_ref(),
          signer_bump
      ][..]];
  
      thaw(
          FreezeParams {
              delegate: ctx.accounts.loan_account.to_account_info(),
              token_account: ctx.accounts.deposit_token_account.to_account_info(),
              edition: ctx.accounts.edition.to_account_info(),
              mint: ctx.accounts.mint.to_account_info(),
              signer_seeds: signer_seeds
          }
      )?;
  }

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