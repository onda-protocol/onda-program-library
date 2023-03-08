use {
    anchor_lang::{
        prelude::*,
    },
    anchor_spl::token::{Mint},
    mpl_token_metadata::{
        state::Metadata,
    },
};

use crate::constants::*;
use crate::utils::{assert_metadata_valid};
use crate::state::{Collection, Config};
use crate::error::ErrorCodes;


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
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_collection(
    ctx: Context<InitCollection>,
    config: Config,
) -> Result<()> {
    let collection = &mut ctx.accounts.collection;
    
    require_keys_eq!(ctx.accounts.authority.key(), ADMIN_PUBKEY);

    assert_metadata_valid(&ctx.accounts.metadata, &ctx.accounts.mint.to_account_info())?;

    let metadata = Metadata::deserialize(
        &mut ctx.accounts.metadata.data.borrow_mut().as_ref()
    )?;

    match metadata.collection {
        Some(collection) => {
            if collection.verified != true {
                return err!(ErrorCodes::InvalidCollection);
            }
        },
        None => return err!(ErrorCodes::InvalidCollection),
    }
    
    collection.authority = ctx.accounts.authority.key();
    collection.mint = ctx.accounts.mint.key();
    collection.bump = *ctx.bumps.get("collection").unwrap();
    collection.config = config;

    Ok(())
}