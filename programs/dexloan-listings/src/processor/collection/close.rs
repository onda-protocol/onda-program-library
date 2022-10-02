use anchor_lang::{
  prelude::*,
};
use anchor_spl::token::{Mint};
use crate::state::{Collection};
use crate::constants::*;

#[derive(Accounts)]
pub struct CloseCollection<'info> {
  #[account(
      constraint = signer.key() == SIGNER_PUBKEY
  )]
  pub signer: Signer<'info>,
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(
      mut,
      seeds = [
          Collection::PREFIX,
          mint.key().as_ref(),
      ],
      bump,
      close = authority,
  )]
  pub collection: Box<Account<'info, Collection>>,
  pub mint: Box<Account<'info, Mint>>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

pub fn handle_close_collection(
  ctx: Context<CloseCollection>
) -> Result<()> {
  let collection = &mut ctx.accounts.collection;
  
  require_keys_eq!(ctx.accounts.authority.key(), collection.authority);

  Ok(())
}