use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Listing, ListingState, Loan, LoanState};
use crate::error::{ErrorCode};

pub fn close(ctx: Context<CloseListing>) -> Result<()> {
  let listing = &mut ctx.accounts.loan_account;

  listing.close(ctx.accounts.borrower.to_account_info())?;

  Ok(())
}

pub fn migrate(ctx: Context<MigrateListing>) -> Result<()> {
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

#[derive(Accounts)]
#[instruction(amount: u64, basis_points: u32, duration: u64)]
pub struct MigrateListing<'info> {
    /// The person who is listing the loan
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.mint == mint.key(),
        constraint = deposit_token_account.owner == listing_account.borrower.key(),
        constraint = deposit_token_account.amount == 1
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub listing_account: Account<'info, Listing>,

    /// The new listing account
    #[account(
        init,
        payer = payer,
        seeds = [
            Loan::PREFIX,
            listing_account.mint.as_ref(),
            listing_account.borrower.as_ref(),
        ],
        bump,
        space = Loan::space(),
    )]
    pub loan_account: Account<'info, Loan>,
    /// This is where we'll store the borrower's token
    #[account(
        init_if_needed,
        payer = payer,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow_account,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}