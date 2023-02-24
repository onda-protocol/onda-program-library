use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Rental, Collection};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct WithdrawFromRentalEscrow<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(
        mut,
        seeds = [
            Rental::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
        has_one = mint,
        has_one = lender,
    )]
    pub rental: Account<'info, Rental>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
    )]
    pub collection: Box<Account<'info, Collection>>,
    /// CHECK: constrained by seeds
    #[account(
        mut,
        seeds = [
            Rental::ESCROW_PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
    )]
    pub rental_escrow: AccountInfo<'info>,  
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}


pub fn handle_withdraw_from_rental_escrow<'info>(ctx: Context<'_, '_, '_, 'info, WithdrawFromRentalEscrow<'info>>) -> Result<()> {
    withdraw_from_rental_escrow(
        &mut ctx.accounts.rental,
        &mut ctx.accounts.rental_escrow,
        &ctx.accounts.lender,
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.metadata.to_account_info(),
        &mut ctx.remaining_accounts.iter(),
        ctx.accounts.clock.unix_timestamp,
    )?;

    Ok(())
}