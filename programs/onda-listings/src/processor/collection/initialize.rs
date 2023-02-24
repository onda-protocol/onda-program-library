use anchor_lang::{
    prelude::*,
};
use anchor_spl::token::{Mint};
use crate::state::{Collection, Config};
use crate::constants::*;

#[derive(Accounts)]
#[instruction(config: Config)]
pub struct InitCollection<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [
            Collection::PREFIX,
            mint.key().as_ref(),
        ],
        bump,
        payer = authority,
        space = Collection::space(),
    )]
    pub collection: Box<Account<'info, Collection>>,
    pub mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_collection(
    ctx: Context<InitCollection>,
    config: Config,
) -> Result<()> {
    let collection = &mut ctx.accounts.collection;
    
    require_keys_eq!(ctx.accounts.authority.key(), ADMIN_PUBKEY);
    
    collection.authority = ctx.accounts.authority.key();
    collection.mint = ctx.accounts.mint.key();
    collection.bump = *ctx.bumps.get("collection").unwrap();
    collection.config = config;

    Ok(())
}