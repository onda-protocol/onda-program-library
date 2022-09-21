use anchor_lang::{
    prelude::*,
};
use anchor_spl::token::{Mint};
use solana_program::pubkey;
use crate::state::{Collection};

#[derive(Accounts)]
pub struct InitCollection<'info> {
    #[account(
        constraint = signer.key() == pubkey!("4RfijtGGJnnaLYYByWGTbkPrGgvmKeAP1bZBhwZApLPq")
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
    ctx: Context<InitCollection>
) -> Result<()> {
    let collection = &mut ctx.accounts.collection;
    
    let admin_pubkey = pubkey!("AH7F2EPHXWhfF5yc7xnv1zPbwz3YqD6CtAqbCyE9dy7r");
    require_keys_eq!(ctx.accounts.authority.key(), admin_pubkey);
    
    collection.authority = ctx.accounts.authority.key();
    collection.mint = ctx.accounts.mint.key();
    collection.bump = *ctx.bumps.get("collection").unwrap();

    Ok(())
}