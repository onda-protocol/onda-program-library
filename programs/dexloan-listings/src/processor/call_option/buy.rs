use anchor_lang::{
  prelude::*,
  solana_program::program_option::{COption}
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionState, TokenManager};

#[derive(Accounts)]
pub struct BuyCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
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
        constraint = call_option_account.seller != buyer.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == CallOptionState::Listed,
    )]
    pub call_option_account: Account<'info, CallOption>,   
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager_account: Account<'info, TokenManager>, 
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.delegate == COption::Some(call_option_account.key()),
        associated_token::mint = mint,
        associated_token::authority = seller,
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

pub fn handle_buy_call_option(ctx: Context<BuyCallOption>) -> Result<()> {
  let call_option = &mut ctx.accounts.call_option_account;

  call_option.state = CallOptionState::Active;
  call_option.buyer = ctx.accounts.buyer.key();

  // Transfer option cost
  anchor_lang::solana_program::program::invoke(
      &anchor_lang::solana_program::system_instruction::transfer(
          &call_option.buyer,
          &call_option.seller,
          call_option.amount,
      ),
      &[
          ctx.accounts.seller.to_account_info(),
          ctx.accounts.buyer.to_account_info(),
      ]
  )?;

  Ok(())
}