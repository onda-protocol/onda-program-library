use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, Collection, TokenManager};
use crate::error::{ErrorCodes};
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
        ],
        space = TokenManager::space(),
        bump,
        constraint = (
            token_manager.authority == Some(seller.key()) || 
            token_manager.authority == None
        ) @ ErrorCodes::Unauthorized,
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
    #[account(mut)]
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub token_record: Option<UncheckedAccount<'info>>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules: Option<UncheckedAccount<'info>>, 
    /// Misc
    /// CHECK: validated in cpi
    pub sysvar_instructions: UncheckedAccount<'info>,
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
    let seller = &ctx.accounts.seller;
    let token_manager = &mut ctx.accounts.token_manager;
    let collection = &ctx.accounts.collection;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let token_record = &ctx.accounts.token_record;
    let mint = &ctx.accounts.mint;
    let metadata = &ctx.accounts.metadata;
    let edition = &ctx.accounts.edition;
    let token_program = &ctx.accounts.token_program;
    let system_program = &ctx.accounts.system_program;
    let sysvar_instructions = &ctx.accounts.sysvar_instructions;
    let authorization_rules_program = &ctx.accounts.authorization_rules_program;
    let authorization_rules = &ctx.accounts.authorization_rules;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    if unix_timestamp > expiry {
        return Err(ErrorCodes::InvalidExpiry.into())
    }

    require_eq!(token_manager.accounts.loan, false, ErrorCodes::InvalidState);

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
    token_manager.authority = Some(seller.key());
    token_manager.accounts.call_option = true;
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    // Freeze deposit token account
    if deposit_token_account.delegate.is_some() {
        if deposit_token_account.delegate.unwrap() != token_manager.key() {
            return err!(ErrorCodes::InvalidState);
        }
    } else {
        handle_delegate_and_freeze(
            token_manager,
            seller.to_account_info(),
            deposit_token_account.to_account_info(),
            match token_record {
                Some(token_record) => Some(token_record.to_account_info()),
                None => None,
            },
            mint.to_account_info(),
            metadata.to_account_info(),
            edition.to_account_info(),
            token_program.to_account_info(),
            system_program.to_account_info(),
            sysvar_instructions.to_account_info(),
            authorization_rules_program.to_account_info(),
            match authorization_rules {
                Some(authorization_rules) => Some(authorization_rules.to_account_info()),
                None => None,
            }
        )?;
    }

    Ok(())
}

  