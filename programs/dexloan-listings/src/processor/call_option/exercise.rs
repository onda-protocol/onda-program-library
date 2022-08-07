use anchor_lang::{
  prelude::*,
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionState};
use crate::error::{DexloanError};
use crate::utils::{pay_creator_fees, thaw, FreezeParams};

#[derive(Accounts)]
pub struct ExerciseCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        constraint = call_option_account.buyer == buyer.key(),
        constraint = call_option_account.state == CallOptionState::Active,
        bump,
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}


pub fn handle_exercise_call_option<'info>(ctx: Context<'_, '_, '_, 'info, ExerciseCallOption<'info>>) -> Result<()> {
  let call_option = &mut ctx.accounts.call_option_account;
  let unix_timestamp = ctx.accounts.clock.unix_timestamp;

  msg!("Exercise with strike price: {} lamports", call_option.strike_price);

  if unix_timestamp > call_option.expiry {
      return Err(DexloanError::OptionExpired.into())
  }

  call_option.state = CallOptionState::Exercised;

  let signer_bump = &[call_option.bump];
  let signer_seeds = &[&[
      CallOption::PREFIX,
      call_option.mint.as_ref(),
      call_option.seller.as_ref(),
      signer_bump
  ][..]];

  thaw(
      FreezeParams {
          delegate: call_option.to_account_info(),
          token_account: ctx.accounts.deposit_token_account.to_account_info(),
          edition: ctx.accounts.edition.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          signer_seeds,
      }
  )?;

  let remaining_amount = pay_creator_fees(
      &mut ctx.remaining_accounts.iter(),
      call_option.strike_price,
      &ctx.accounts.mint.to_account_info(),
      &ctx.accounts.metadata.to_account_info(),
      &ctx.accounts.buyer.to_account_info(),
  )?;

  anchor_lang::solana_program::program::invoke(
      &anchor_lang::solana_program::system_instruction::transfer(
          &call_option.buyer,
          &call_option.seller,
          remaining_amount,
      ),
      &[
          ctx.accounts.buyer.to_account_info(),
          ctx.accounts.seller.to_account_info(),
      ]
  )?;

  anchor_spl::token::transfer(
      CpiContext::new_with_signer(
          ctx.accounts.token_program.to_account_info(),
          anchor_spl::token::Transfer {
              from: ctx.accounts.deposit_token_account.to_account_info(),
              to: ctx.accounts.buyer_token_account.to_account_info(),
              authority: call_option.to_account_info(),
          },
          signer_seeds
      ),
      1
  )?;
  
  Ok(())
}