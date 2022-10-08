use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionBid, CallOptionState, Collection, TokenManager};
use crate::utils::*;
use crate::error::*;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct SellCallOption<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub buyer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.mint == mint.key(),
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = seller,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        space = CallOption::space(),
        bump,
    )]
    pub call_option: Box<Account<'info, CallOption>>, 
    #[account(
        mut,
        seeds = [
            CallOptionBid::PREFIX,
            collection.mint.as_ref(),
            buyer.key().as_ref(),
            &[id],
        ],
        close = buyer,
        bump,
    )]
    pub call_option_bid: Box<Account<'info, CallOptionBid>>,
    #[account(
        mut,
        seeds=[
            CallOptionBid::VAULT_PREFIX,
            call_option_bid.key().as_ref()
        ],
        bump,
    )]
    pub escrow_payment_account: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = seller,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref()
        ],
        space = TokenManager::space(),
        bump,
    )]
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
    )]
    pub collection: Box<Account<'info, Collection>>,
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
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_sell_call_option(
  ctx: Context<SellCallOption>,
  _bid_id: u8,
) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option;
    let bid = ctx.accounts.call_option_bid;
    let seller = ctx.accounts.seller;
    let escrow_payment_account = ctx.accounts.escrow_payment_account;
    let token_manager = &mut ctx.accounts.token_manager;
    let deposit_token_account = *ctx.accounts.deposit_token_account;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    require_eq!(token_manager.accounts.loan, false, DexloanError::InvalidState);

    // Init
    call_option.seller = ctx.accounts.seller.key();
    call_option.mint = ctx.accounts.mint.key();
    call_option.bump = *ctx.bumps.get("call_option").unwrap();
    //
    call_option.amount = bid.amount;
    call_option.expiry = bid.expiry;
    call_option.strike_price = bid.strike_price;
    call_option.state = CallOptionState::Active;
    //
    token_manager.accounts.call_option = true;
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    maybe_delegate_and_freeze_token_account(
        token_manager,
        deposit_token_account,
        ctx.accounts.seller.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.seller.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    let signer_seeds = &[&[
        CallOptionBid::VAULT_PREFIX,
        bid.key().as_ref(),
        &[bid.escrow_bump]
    ][..]];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::system_instruction::transfer(
            &escrow_payment_account.key(),
            &call_option.seller,
            call_option.amount,
        ),
        &[
            escrow_payment_account.to_account_info(),
            ctx.accounts.seller.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}