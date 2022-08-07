use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Hire, HireState};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(Accounts)]
pub struct RecoverHire<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(mut)]
    /// CHECK: validated in constraints
    pub borrower: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower
    )]
    pub hire_token_account: Account<'info, TokenAccount>,
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
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_recover_hire(ctx: Context<RecoverHire>) -> Result<()> {
    let hire = &mut ctx.accounts.hire_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    if !hire.current_expiry.is_some() {
        return err!(DexloanError::NumericalOverflow)
    }

    let current_expiry = hire.current_expiry.unwrap();
    msg!("current_expiry {}", current_expiry);
    msg!("unix_timestamp {}", unix_timestamp);
    if current_expiry > unix_timestamp {
        return Err(DexloanError::NotExpired.into());
    }

    hire.current_expiry = None;
    hire.borrower = None;
    hire.state = HireState::Listed;

    let signer_bump = &[hire.bump];
    let signer_seeds = &[&[
        Hire::PREFIX,
        hire.mint.as_ref(),
        hire.lender.as_ref(),
        signer_bump
    ][..]];

    // Thaw & Transfer NFT back to deposit account
    thaw(
        FreezeParams {
            delegate: hire.to_account_info(),
            token_account: ctx.accounts.hire_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            signer_seeds: signer_seeds
        }
    )?;
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.hire_token_account.to_account_info(),
                to: ctx.accounts.deposit_token_account.to_account_info(),
                authority: hire.to_account_info(),
            },
            signer_seeds
        ),
        1
    )?;

    // Delegate authority & freeze deposit token account again
    anchor_spl::token::approve(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Approve {
                to: ctx.accounts.deposit_token_account.to_account_info(),
                delegate: hire.to_account_info(),
                authority: ctx.accounts.lender.to_account_info(),
            }
        ),
        1
    )?;

    freeze(
        FreezeParams {
            delegate: hire.to_account_info(),
            token_account: ctx.accounts.deposit_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            signer_seeds: signer_seeds
        }
    )?;

    Ok(())
}