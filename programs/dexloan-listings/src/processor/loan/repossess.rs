use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(Accounts)]
pub struct RepossessCollateral<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub lender_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        constraint = loan_account.borrower == borrower.key()
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
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_repossess_collateral(ctx: Context<RepossessCollateral>) -> Result<()> {
  let loan = &mut ctx.accounts.loan_account;

  let unix_timestamp = ctx.accounts.clock.unix_timestamp as u64;
  let loan_start_date = loan.start_date as u64;
  let loan_duration = unix_timestamp - loan_start_date;

  msg!("Loan start date: {} seconds", loan_start_date);
  msg!("Loan duration: {} seconds", loan.duration);
  msg!("Time passed: {} seconds", loan_duration);

  if loan.duration > loan_duration  {
      return Err(DexloanError::NotOverdue.into())
  }
  
  loan.state = LoanState::Defaulted;

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

  // Transfer NFT
  anchor_spl::token::transfer(
      CpiContext::new_with_signer(
          ctx.accounts.token_program.to_account_info(),
          anchor_spl::token::Transfer {
              from: ctx.accounts.deposit_token_account.to_account_info(),
              to: ctx.accounts.lender_token_account.to_account_info(),
              authority: loan.to_account_info(),
          },
          signer_seeds
      ),
      1
  )?;

  Ok(())
}