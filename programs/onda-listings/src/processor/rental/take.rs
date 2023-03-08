use anchor_lang::{system_program,prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Rental, RentalState, TokenManager};
use crate::error::{ErrorCodes};
use crate::constants::*;
use crate::utils::*;

#[derive(Accounts)]
#[instruction(days: u16)]
pub struct TakeRental <'info> {
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
    )]
    pub rental: Box<Account<'info, Rental>>,
    /// CHECK: constrained by seeds
    #[account(
        init_if_needed,
        seeds = [
            Rental::ESCROW_PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
        payer = borrower,
        owner = system_program::ID,
        space = 0,
    )]
    pub rental_escrow: AccountInfo<'info>,   
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
            TokenManager::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,  
    #[account(constraint = mint.supply == 1)]
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_take_rental<'info>(ctx: Context<'_, '_, '_, 'info, TakeRental<'info>>, days: u16) -> Result<()> {
    let rental = &mut ctx.accounts.rental;
    let token_manager = &mut ctx.accounts.token_manager;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    msg!("escrow balance is {}", rental.escrow_balance);

    if ctx.bumps.get("rental_escrow").is_some() {
        rental.escrow_bump = *ctx.bumps.get("rental_escrow").unwrap();
    }

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

    if rental.borrower.is_some() {
        require_keys_eq!(rental.borrower.unwrap(), ctx.accounts.borrower.key());
    } else {
        rental.borrower = Some(ctx.accounts.borrower.key());
    }

    let duration = i64::from(days) * SECONDS_PER_DAY;
    let current_expiry = unix_timestamp + duration;

    if current_expiry > rental.expiry {
        return err!(ErrorCodes::InvalidExpiry)
    }

    rental.current_start = Some(unix_timestamp);
    rental.current_expiry = Some(current_expiry);
    rental.state = RentalState::Rented;

    if rental.amount > 0 {
        process_payment_to_rental_escrow(
            rental,
            ctx.accounts.rental_escrow.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
            days
        )?;
    }

    thaw_and_transfer_from_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.lender.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.rental_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
    )?;

    delegate_and_freeze_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.rental_token_account.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.lender.to_account_info()
    )?;

    Ok(())
}