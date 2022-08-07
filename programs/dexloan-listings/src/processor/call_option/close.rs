use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionState};
use crate::error::{DexloanError};
use crate::utils::{thaw, FreezeParams};

#[derive(Accounts)]
pub struct CloseCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.mint == mint.key(),
        close = seller
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        mut,
        // constraint = deposit_token_account.delegate == COption::Some(escrow_account.key()),
        associated_token::mint = mint,
        associated_token::authority = seller
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

pub fn handle_close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
  let call_option = &ctx.accounts.call_option_account;
  let unix_timestamp = ctx.accounts.clock.unix_timestamp;

  if call_option.state == CallOptionState::Active {
      if call_option.expiry > unix_timestamp {
          return Err(DexloanError::OptionNotExpired.into())
      }
  }

  if ctx.accounts.deposit_token_account.is_frozen() {
      let signer_bump = &[ctx.accounts.call_option_account.bump];
      let signer_seeds = &[&[
          CallOption::PREFIX,
          call_option.mint.as_ref(),
          call_option.seller.as_ref(),
          signer_bump
      ][..]];
  
      thaw(
          FreezeParams {
              delegate: ctx.accounts.call_option_account.to_account_info(),
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
              authority: ctx.accounts.seller.to_account_info(),
          }
      )
  )?;

  Ok(())
}