use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionState};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(Accounts)]
#[instruction(amount: u64, strike_price: u64, expiry: i64)]
pub struct InitCallOption<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.owner == seller.key(),
        associated_token::mint = mint,
        associated_token::authority = seller,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = seller,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        space = CallOption::space(),
        bump,
    )]
    pub call_option_account: Account<'info, CallOption>,    
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_call_option(
  ctx: Context<InitCallOption>,
  amount: u64,
  strike_price: u64,
  expiry: i64
) -> Result<()> {
  let call_option = &mut ctx.accounts.call_option_account;
  let unix_timestamp = ctx.accounts.clock.unix_timestamp;
  
  msg!("unix_timestamp: {} seconds", unix_timestamp);
  msg!("expiry: {} seconds", expiry);
  
  if unix_timestamp > expiry {
      return Err(DexloanError::InvalidExpiry.into())
  }

  // Init
  call_option.seller = ctx.accounts.seller.key();
  call_option.mint = ctx.accounts.mint.key();
  call_option.bump = *ctx.bumps.get("call_option_account").unwrap();
  //
  call_option.amount = amount;
  call_option.expiry = expiry;
  call_option.strike_price = strike_price;
  call_option.state = CallOptionState::Listed;
  // Delegate authority
  anchor_spl::token::approve(
      CpiContext::new(
          ctx.accounts.token_program.to_account_info(),
          anchor_spl::token::Approve {
              to: ctx.accounts.deposit_token_account.to_account_info(),
              delegate: call_option.to_account_info(),
              authority: ctx.accounts.seller.to_account_info(),
          }
      ),
      1
  )?;

  let signer_bump = &[call_option.bump];
  let signer_seeds = &[&[
      CallOption::PREFIX,
      call_option.mint.as_ref(),
      call_option.seller.as_ref(),
      signer_bump
  ][..]];

  freeze(
      FreezeParams {
          delegate: call_option.to_account_info(),
          token_account: ctx.accounts.deposit_token_account.to_account_info(),
          edition: ctx.accounts.edition.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          signer_seeds: signer_seeds
      }
  )?;

  Ok(())
}