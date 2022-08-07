use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Hire, HireState};
use crate::error::{DexloanError};
use crate::constants::*;
use crate::utils::*;

#[derive(Accounts)]
#[instruction(days: u16)]
pub struct TakeHire <'info> {
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
    )]
    pub hire_account: Account<'info, Hire>,   
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
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_take_hire<'info>(ctx: Context<'_, '_, '_, 'info, TakeHire<'info>>, days: u16) -> Result<()> {
    let hire = &mut ctx.accounts.hire_account;
    let start_date = ctx.accounts.clock.unix_timestamp;

    hire.state = HireState::Hired;

    if hire.borrower.is_some() {
        require_keys_eq!(hire.borrower.unwrap(), ctx.accounts.borrower.key());
    } else {
        hire.borrower = Some(ctx.accounts.borrower.key());
    }

    let duration = i64::from(days) * SECONDS_PER_DAY;
    let current_expiry = start_date + duration;

    if current_expiry > hire.expiry {
        return err!(DexloanError::InvalidExpiry)
    }

    msg!("duration {}", duration);
    hire.current_expiry = Some(current_expiry);


    if hire.amount > 0 {
        let amount = u64::from(days) * hire.amount;
        msg!("amount {}", amount);

        let remaining_amount = pay_creator_fees(
            &mut ctx.remaining_accounts.iter(),
            amount,
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.metadata.to_account_info(),
            &ctx.accounts.borrower.to_account_info(),
        )?;
    
        // Transfer fee
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
    }

    // Thaw & Transfer NFT to hire account
    let signer_bump = &[hire.bump];
    let signer_seeds = &[&[
        Hire::PREFIX,
        hire.mint.as_ref(),
        hire.lender.as_ref(),
        signer_bump
    ][..]];

    thaw(
        FreezeParams {
            delegate: hire.to_account_info(),
            token_account: ctx.accounts.deposit_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            signer_seeds,
        }
    )?;
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.deposit_token_account.to_account_info(),
                to: ctx.accounts.hire_token_account.to_account_info(),
                authority: hire.to_account_info(),
            },
            signer_seeds
        ),
        1
    )?;

    // Delegate authority & freeze hire token account
    anchor_spl::token::approve(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Approve {
                to: ctx.accounts.hire_token_account.to_account_info(),
                delegate: hire.to_account_info(),
                authority: ctx.accounts.borrower.to_account_info(),
            }
        ),
        1
    )?;

    freeze(
        FreezeParams {
            delegate: hire.to_account_info(),
            token_account: ctx.accounts.hire_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            signer_seeds: signer_seeds
        }
    )?;

    Ok(())
}