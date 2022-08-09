use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState, TokenManager};
use crate::utils::*;

#[derive(Accounts)]
pub struct CloseLoan<'info> {
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = loan_account.bump,
        constraint = loan_account.borrower == *borrower.key,
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed || loan_account.state == LoanState::Defaulted,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager_account: Account<'info, TokenManager>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_close_loan(ctx: Context<CloseLoan>) -> Result<()> {
    let token_manager = &mut ctx.accounts.token_manager_account;

    thaw_and_revoke_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
    )?;
  
    Ok(())
}