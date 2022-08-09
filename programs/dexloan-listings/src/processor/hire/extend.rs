use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Hire, HireState, TokenManager};
use crate::constants::*;
use crate::error::*;
use crate::utils::*;

#[derive(Accounts)]
#[instruction(days: u16)]
pub struct ExtendHire<'info> {
    #[account(mut)]
    /// CHECK: validated seeds constraints
    pub lender: AccountInfo<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        seeds = [
            Hire::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        bump,
        has_one = mint,
        has_one = lender,
        constraint = hire.state == HireState::Hired,
        constraint = hire.borrower.is_some() && hire.borrower.unwrap() == borrower.key(), 
    )]
    pub hire: Box<Account<'info, Hire>>,   
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
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_extend_hire<'info>(ctx: Context<'_, '_, '_, 'info, ExtendHire<'info>>, days: u16) -> Result<()> {
    let hire = &mut ctx.accounts.hire;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    require!(hire.current_start.is_some(), DexloanError::InvalidState);
    require!(hire.current_expiry.is_some(), DexloanError::InvalidState);

    if hire.escrow_balance > 0 {
        withdraw_from_escrow_balance(
            hire,
            ctx.accounts.lender.to_account_info(),
            unix_timestamp,
        )?;
    }

    let amount = u64::from(days) * hire.amount;
    let duration = i64::from(days) * SECONDS_PER_DAY;
    let current_expiry = hire.current_expiry.unwrap();
    let new_current_expiry = if current_expiry > unix_timestamp {
        current_expiry + duration
    } else {
        unix_timestamp + duration
    };
    
    hire.current_expiry = Some(new_current_expiry);
    hire.current_start = Some(unix_timestamp);

    let remaining_amount = pay_creator_fees(
        &mut ctx.remaining_accounts.iter(),
        amount,
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.metadata.to_account_info(),
        &ctx.accounts.borrower.to_account_info(),
    )?;

    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &hire.borrower.unwrap(),
            &hire.lender,
            remaining_amount,
        ),
        &[
            ctx.accounts.borrower.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ]
    )?;

    Ok(())
}