use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(Accounts)]
pub struct RepossessCollateral<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub lender_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        constraint = loan_account.borrower == borrower.key()
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        bump,
        constraint = token_manager_account.accounts.hire == false,
        constraint = token_manager_account.accounts.loan == true,
    )]   
    pub token_manager_account: Account<'info, TokenManager>,
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
  let token_manager = &mut ctx.accounts.token_manager_account;

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
  token_manager.accounts.loan = false;

  thaw_and_transfer_from_token_account(
    token_manager,
    ctx.accounts.token_program.to_account_info(),
    ctx.accounts.deposit_token_account.to_account_info(),
    ctx.accounts.lender_token_account.to_account_info(),
    ctx.accounts.mint.to_account_info(),
    ctx.accounts.edition.to_account_info()
  )?;

  Ok(())
}