use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{Hire, HireState};
use crate::constants::*;
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
      constraint = hire_account.state == HireState::Hired,
      constraint = hire_account.borrower.is_some() && hire_account.borrower.unwrap() == borrower.key(), 
  )]
  pub hire_account: Account<'info, Hire>,   
  #[account(constraint = mint.supply == 1)]
  pub mint: Account<'info, Mint>,
  /// CHECK: deserialized and checked
  pub metadata: UncheckedAccount<'info>,
  /// Misc
  pub system_program: Program<'info, System>,
  pub token_program: Program<'info, Token>,
  pub clock: Sysvar<'info, Clock>,
}

pub fn handle_extend_hire<'info>(ctx: Context<'_, '_, '_, 'info, ExtendHire<'info>>, days: u16) -> Result<()> {
    let hire = &mut ctx.accounts.hire_account;

    let amount = u64::from(days) * hire.amount;
    let duration = i64::from(days) * SECONDS_PER_DAY;

    let new_current_expiry = hire.current_expiry.unwrap() + duration;
    hire.current_expiry = Some(new_current_expiry);

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