use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Rental};
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
    /// The listing the loan is being issued against
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
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}


pub fn handle_withdraw_from_rental_escrow(ctx: Context<WithdrawFromRentalEscrow>) -> Result<()> {
    let rental = &mut ctx.accounts.rental;

    withdraw_from_rental_escrow(
        rental,
        &ctx.accounts.rental_escrow.to_account_info(),
        &ctx.accounts.lender.to_account_info(),
        ctx.accounts.clock.unix_timestamp,
    )?;

    Ok(())
}