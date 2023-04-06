use anchor_lang::{
  prelude::*,
};
use anchor_spl::{token::{Mint, Token, TokenAccount}, associated_token::{AssociatedToken}};
use crate::state::{CallOption, CallOptionState, TokenManager};
use crate::error::{ErrorCodes};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct ExerciseCallOption<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump,
        has_one = mint,
        has_one = seller,
        constraint = call_option.buyer.unwrap() == buyer.key(),
        constraint = call_option.state == CallOptionState::Active,
    )]
    pub call_option: Box<Account<'info, CallOption>>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref()
        ],
        bump,
        constraint = token_manager.accounts.rental != true @ ErrorCodes::InvalidState,
        constraint = token_manager.authority.unwrap() == seller.key() @ ErrorCodes::Unauthorized,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: validated in cpi
    pub deposit_token_record: Option<UncheckedAccount<'info>>,
    #[account(
        init,
        seeds = [
            TokenManager::ESCROW_PREFIX,
            token_manager.key().as_ref(),
        ],
        bump,
        payer = buyer,
        token::mint = mint,
        token::authority = token_manager,
    )]
    pub escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: contrained on loan_account
    pub escrow_token_record: Option<UncheckedAccount<'info>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules: Option<UncheckedAccount<'info>>, 
    /// Misc
    /// CHECK: not supported by anchor? used in cpi
    pub sysvar_instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}


pub fn handle_exercise_call_option<'info>(ctx: Context<'_, '_, '_, 'info, ExerciseCallOption<'info>>) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option;
    let token_manager = &mut ctx.accounts.token_manager;
    let seller = &mut ctx.accounts.seller;
    let buyer = &mut ctx.accounts.buyer;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let deposit_token_record = &mut ctx.accounts.deposit_token_record;
    let escrow_token_account = &mut ctx.accounts.escrow_token_account;
    let escrow_token_record = &mut ctx.accounts.escrow_token_record;
    let mint = &ctx.accounts.mint;
    let edition = &ctx.accounts.edition;
    let metadata_info = &mut ctx.accounts.metadata;
    let authorization_rules_program = &mut ctx.accounts.authorization_rules_program;
    let authorization_rules = &mut ctx.accounts.authorization_rules;
    let token_program = &ctx.accounts.token_program;
    let associated_token_program = &ctx.accounts.associated_token_program;
    let system_program = &ctx.accounts.system_program;
    let sysvar_instructions = &ctx.accounts.sysvar_instructions;
    let remaining_accounts = &mut ctx.remaining_accounts.iter();
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    msg!("Exercise with strike price: {} lamports", call_option.strike_price);

    if unix_timestamp > call_option.expiry {
        return Err(ErrorCodes::OptionExpired.into())
    }

    call_option.state = CallOptionState::Exercised;
    token_manager.accounts.call_option = false;
    token_manager.accounts.rental = false;

    let remaining_amount = pay_creator_royalties(
        call_option.strike_price,
        &mint.to_account_info(),
        &metadata_info.to_account_info(),
        &mut buyer.to_account_info(),
        remaining_accounts,
    )?;  

    msg!("remaining amount {}", remaining_amount);
    msg!("paid to creators {}", call_option.strike_price - remaining_amount);

    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &call_option.buyer.unwrap(),
            &call_option.seller,
            remaining_amount,
        ),
        &[
            buyer.to_account_info(),
            seller.to_account_info(),
        ]
    )?;

    handle_thaw_and_transfer(
        token_manager,
        buyer.to_account_info(),
        deposit_token_account.to_account_info(),
        match deposit_token_record {
            Some(account) => Some(account.to_account_info()),
            None => None,
        },
        escrow_token_account.to_account_info(),
        match escrow_token_record {
            Some(account) => Some(account.to_account_info()),
            None => None,
        },
        buyer.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        token_program.to_account_info(),
        associated_token_program.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        authorization_rules_program.to_account_info(),
        match authorization_rules {
            Some(account) => Some(account.to_account_info()),
            None => None,
        },
    )?;
  
    Ok(())
}