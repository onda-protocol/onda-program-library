use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Rental, RentalState, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct RecoverRental<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(mut)]
    /// CHECK: validated in constraints
    pub borrower: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower
    )]
    pub rental_token_account: Box<Account<'info, TokenAccount>>,
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
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_recover_rental<'info>(ctx: Context<'_, '_, '_, 'info, RecoverRental<'info>>) -> Result<()> {
    let rental = &mut ctx.accounts.rental;
    let token_manager = &mut ctx.accounts.token_manager;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    require!(rental.current_start.is_some(), DexloanError::InvalidState);
    require!(rental.current_expiry.is_some(), DexloanError::InvalidState);

    let current_expiry = rental.current_expiry.unwrap();

    if current_expiry > unix_timestamp {
        return Err(DexloanError::NotExpired.into());
    }

    thaw_and_transfer_from_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.lender.to_account_info(),
        ctx.accounts.rental_token_account.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
    )?;


    delegate_and_freeze_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.lender.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.lender.to_account_info(),
    )?;

    if rental.escrow_balance > 0 {
        withdraw_from_rental_escrow(
            rental,
            &mut ctx.accounts.rental_escrow,
            &ctx.accounts.lender,
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.metadata.to_account_info(),
            &mut ctx.remaining_accounts.iter(),
            ctx.accounts.clock.unix_timestamp,
        )?;
    }

    rental.current_start = None;
    rental.current_expiry = None;
    rental.borrower = None;
    rental.state = RentalState::Listed;

    Ok(())
}