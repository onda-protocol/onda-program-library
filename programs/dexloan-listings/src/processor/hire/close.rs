use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Hire, HireState};
use crate::utils::*;

#[derive(Accounts)]
pub struct CloseHire<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub lender: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            Hire::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
        close = lender,
        constraint = hire_account.borrower == None,
        constraint = hire_account.state != HireState::Hired
    )]
    pub hire_account: Account<'info, Hire>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}


pub fn handle_close_hire(ctx: Context<CloseHire>) -> Result<()> {
  let hire = &mut ctx.accounts.hire_account;

  let signer_bump = &[hire.bump];
  let signer_seeds = &[&[
      Hire::PREFIX,
      hire.mint.as_ref(),
      hire.lender.as_ref(),
      signer_bump
  ][..]];

  thaw(
      FreezeParams {
          delegate: hire.to_account_info(),
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
              authority: ctx.accounts.lender.to_account_info(),
          }
      )
  )?;

  Ok(())
}