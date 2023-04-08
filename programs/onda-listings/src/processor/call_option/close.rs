use {
    anchor_lang::{
        prelude::*,
        solana_program::{
            program::{invoke_signed},
            system_instruction::{transfer}
        },
        AccountsClose
    },
    anchor_spl::token::{Mint, Token, TokenAccount}
};
use crate::state::{CallOption, CallOptionBid, CallOptionState, Collection, TokenManager};
use crate::error::{ErrorCodes};
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
        ],
        bump,
        constraint = token_manager.authority.unwrap() == seller.key() @ ErrorCodes::Unauthorized,
    )]   
    pub token_manager: Account<'info, TokenManager>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub deposit_token_record: Option<UncheckedAccount<'info>>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
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
}

pub fn handle_close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option;
    let token_manager = &mut ctx.accounts.token_manager;
    let seller = &ctx.accounts.seller;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let mint = &ctx.accounts.mint;
    let metadata = &ctx.accounts.metadata;
    let edition = &ctx.accounts.edition;
    let deposit_token_record = &ctx.accounts.deposit_token_record;
    let token_program = &ctx.accounts.token_program;
    let system_program = &ctx.accounts.system_program;
    let sysvar_instructions = &ctx.accounts.sysvar_instructions;
    let authorization_rules_program = &ctx.accounts.authorization_rules_program;
    let authorization_rules = &ctx.accounts.authorization_rules;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    if call_option.state == CallOptionState::Active {
        if call_option.expiry > unix_timestamp {
            return Err(ErrorCodes::OptionNotExpired.into())
        }
    }

    token_manager.accounts.call_option = false;

    // IMPORTANT CHECK!
    // The token manager authority is moved to the buyer in the event of exercise
    // We only want to thaw and revoke if the authority is still the seller 
    if token_manager.authority.unwrap().eq(&seller.key()) {
        // IMPORTANT CHECK!
        if token_manager.accounts.rental == false {
            handle_thaw_and_revoke(
                token_manager,
                seller.to_account_info(),
                deposit_token_account.to_account_info(),
                match deposit_token_record {
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
                },
            )?;
        
            token_manager.close(seller.to_account_info())?;    
        } else {
            token_manager.accounts.loan = false;
        }   
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
    let call_option_bid = &ctx.accounts.call_option_bid;
    let escrow_payment_account = &ctx.accounts.escrow_payment_account;

    let call_option_bid_pubkey = call_option_bid.key();
    let signer_bump = &[call_option_bid.escrow_bump];
    let signer_seeds = &[&[
        CallOptionBid::VAULT_PREFIX,
        call_option_bid_pubkey.as_ref(),
        signer_bump
    ][..]];

    invoke_signed(
        &transfer(
            &escrow_payment_account.key(),
            &call_option_bid.buyer,
            call_option_bid.amount,
        ),
        &[
            escrow_payment_account.to_account_info(),
            ctx.accounts.buyer.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}