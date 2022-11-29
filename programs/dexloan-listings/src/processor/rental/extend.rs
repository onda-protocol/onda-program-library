use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Rental, RentalState, TokenManager};
use crate::constants::*;
use crate::error::*;
use crate::utils::*;

#[derive(Accounts)]
#[instruction(days: u16)]
pub struct ExtendRental<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    /// CHECK: validated seeds constraints
    pub lender: AccountInfo<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
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
        constraint = rental.state == RentalState::Rented,
        constraint = rental.borrower.is_some() && rental.borrower.unwrap() == borrower.key(), 
    )]
    pub rental: Box<Account<'info, Rental>>,
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
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Box<Account<'info, Mint>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_extend_rental<'info>(ctx: Context<'_, '_, '_, 'info, ExtendRental<'info>>, days: u16) -> Result<()> {
    let rental = &mut ctx.accounts.rental;

    require!(rental.current_start.is_some(), DexloanError::InvalidState);
    require!(rental.current_expiry.is_some(), DexloanError::InvalidState);

    let duration = i64::from(days) * SECONDS_PER_DAY;
    let current_expiry = rental.current_expiry.unwrap();
    let new_current_expiry = current_expiry + duration;
    
    rental.current_expiry = Some(new_current_expiry);

    process_payment_to_rental_escrow(
        rental,
        ctx.accounts.rental_escrow.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        days
    )?;

    Ok(())
}