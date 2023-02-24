use anchor_lang::{
  prelude::*,
};
use anchor_spl::token::{Mint};
use crate::state::{Collection, Config};
use crate::constants::*;

#[derive(Accounts)]
#[instruction(config: Config)]
pub struct UpdateCollection<'info> {
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
  )]
  pub collection: Box<Account<'info, Collection>>,
  pub mint: Box<Account<'info, Mint>>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

pub fn handle_update_collection(
  ctx: Context<UpdateCollection>,
  config: Config,
) -> Result<()> {
  let collection = &mut ctx.accounts.collection;
  
  require_keys_eq!(ctx.accounts.authority.key(), collection.authority);
  
  collection.config = config;

  Ok(())
}