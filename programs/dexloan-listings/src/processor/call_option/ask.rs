use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, Collection, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(amount: u64, strike_price: u64, expiry: i64)]
pub struct AskCallOption<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub seller: Signer<'info>,
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
        constraint = collection.config.option_enabled == true
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
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_ask_call_option(
  ctx: Context<AskCallOption>,
  amount: u64,
  strike_price: u64,
  expiry: i64
) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option;
    let token_manager = &mut ctx.accounts.token_manager;
    let collection = &ctx.accounts.collection;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    if unix_timestamp > expiry {
        return Err(DexloanError::InvalidExpiry.into())
    }

    require_eq!(token_manager.accounts.loan, false, DexloanError::InvalidState);

    // Init
    call_option.seller = ctx.accounts.seller.key();
    call_option.mint = ctx.accounts.mint.key();
    call_option.bump = *ctx.bumps.get("call_option").unwrap();
    //
    CallOption::init_ask_state(
        call_option,
        amount,
        collection.config.option_basis_points,
        strike_price,
        expiry
    )?;
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

    Ok(())
}

  