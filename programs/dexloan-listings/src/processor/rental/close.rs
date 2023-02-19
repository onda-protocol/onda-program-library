use anchor_lang::{prelude::*, AccountsClose};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Rental, RentalState, TokenManager};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct CloseRental<'info> {
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
        close = lender,
        has_one = mint,
        has_one = lender,
        constraint = rental.borrower == None,
        constraint = rental.state != RentalState::Rented
    )]
    pub rental: Box<Account<'info, Rental>>,
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
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender,
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}


pub fn handle_close_rental(ctx: Context<CloseRental>) -> Result<()> {
    let token_manager = &mut ctx.accounts.token_manager;

    token_manager.accounts.rental = false;
    // IMPORTANT CHECKS!
    if token_manager.accounts.call_option == false && token_manager.accounts.loan == false {
        thaw_and_revoke_token_account(
            token_manager,
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.deposit_token_account.to_account_info(),
            ctx.accounts.lender.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.edition.to_account_info(),
        )?;

        token_manager.close(&mut ctx.accounts.lender.to_account_info())?;
    }


    Ok(())
}