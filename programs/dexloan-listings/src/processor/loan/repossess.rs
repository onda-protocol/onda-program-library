use anchor_lang::{prelude::*};
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::{Loan, LoanState, Rental, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct Repossess<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
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
    pub lender_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
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
        constraint = token_manager.accounts.rental == false,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    /// CHECK: contrained on loan_account
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_repossess(ctx: Context<Repossess>) -> Result<()> {
  let loan = &mut ctx.accounts.loan;
  let token_manager = &mut ctx.accounts.token_manager;
  
  let unix_timestamp = ctx.accounts.clock.unix_timestamp;
  let start_date = loan.start_date.unwrap();
  let duration = unix_timestamp - start_date;

  if loan.duration > duration  {
      return Err(DexloanError::NotOverdue.into())
  }
  
  loan.state = LoanState::Defaulted;
  token_manager.accounts.loan = false;

  thaw_and_transfer_from_token_account(
    token_manager,
    ctx.accounts.token_program.to_account_info(),
    ctx.accounts.borrower.to_account_info(),
    ctx.accounts.deposit_token_account.to_account_info(),
    ctx.accounts.lender_token_account.to_account_info(),
    ctx.accounts.mint.to_account_info(),
    ctx.accounts.edition.to_account_info()
  )?;

  Ok(())
}

#[derive(Accounts)]
pub struct RepossessWithRental<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
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
        has_one = mint,
        constraint = loan.lender.unwrap() == lender.key(),
        constraint = loan.state == LoanState::Active,
    )]
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        mut,
        seeds = [
            Rental::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        close = borrower
    )]
    pub rental: Box<Account<'info, Rental>>,
    /// CHECK: constrained by seeds
    #[account(
        mut,
        seeds = [
            Rental::ESCROW_PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
    )]
    pub rental_escrow: AccountInfo<'info>,  
    #[account(
        mut,
        constraint = token_account.mint == mint.key(),
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,
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

pub fn handle_repossess_with_rental<'info>(ctx: Context<'_, '_, '_, 'info, RepossessWithRental<'info>>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let rental = &mut ctx.accounts.rental;
    let token_manager = &mut ctx.accounts.token_manager;
    let remaining_accounts = &mut ctx.remaining_accounts.iter();
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    let start_date = loan.start_date.unwrap();
    let duration = unix_timestamp - start_date;

    if loan.duration > duration  {
        return err!(DexloanError::NotOverdue);
    }

    loan.state = LoanState::Defaulted;
    token_manager.accounts.loan = false;
    token_manager.accounts.rental = false;

    if rental.borrower.is_some() {
        settle_rental_escrow_balance(
            rental,
            remaining_accounts,
            &ctx.accounts.rental_escrow.to_account_info(),
            &ctx.accounts.borrower.to_account_info(),
            unix_timestamp,
        )?;
    }

    thaw_and_transfer_from_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        ctx.accounts.token_account.to_account_info(),
        ctx.accounts.lender_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
    )?;

    Ok(())
}