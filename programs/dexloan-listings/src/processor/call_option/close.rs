use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionBid, CallOptionState, Collection, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct CloseCallOption<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
        mut,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump,
        has_one = seller,
        has_one = mint,
        close = seller
    )]
    pub call_option: Account<'info, CallOption>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager: Account<'info, TokenManager>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
    let call_option = &ctx.accounts.call_option;
    let token_manager = &mut ctx.accounts.token_manager;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    if call_option.state == CallOptionState::Active {
        if call_option.expiry > unix_timestamp {
            return Err(DexloanError::OptionNotExpired.into())
        }
    }

    token_manager.accounts.call_option = false;
    // IMPORTANT CHECK!
    if token_manager.accounts.hire == true {
        return Ok(());
    }

    if ctx.accounts.deposit_token_account.is_frozen() {
        thaw_and_revoke_token_account(
            token_manager,
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.deposit_token_account.to_account_info(),
            ctx.accounts.seller.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.edition.to_account_info()
        )?;
    } else {
        anchor_spl::token::revoke(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Revoke {
                    source: ctx.accounts.deposit_token_account.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                }
            )
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct CloseCallOptionBid<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        init,
        seeds = [
            CallOptionBid::PREFIX,
            collection.mint.as_ref(),
            buyer.key().as_ref(),
            &[id],
        ],
        payer = buyer,
        space = CallOptionBid::space(),
        bump,
    )]
    pub call_option_bid: Box<Account<'info, CallOptionBid>>,
    #[account(
        init_if_needed,
        seeds=[
            CallOptionBid::VAULT_PREFIX,
            call_option_bid.key().as_ref()
        ],
        payer = buyer,
        space = 0,
        bump,
    )]
    /// CHECK: seeds
    pub escrow_payment_account: UncheckedAccount<'info>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
    )]
    pub collection: Box<Account<'info, Collection>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_close_call_option_bid(ctx: Context<CloseCallOptionBid>, _id: u8) -> Result<()> {
    transfer_from_escrow(
        &mut ctx.accounts.escrow_payment_account.to_account_info(),
        &mut ctx.accounts.buyer.to_account_info(),
        ctx.accounts.escrow_payment_account.lamports()
    )?;

    Ok(())
}