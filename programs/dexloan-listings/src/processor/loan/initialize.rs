use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState, TokenManager};
use crate::utils::*;

#[derive(Accounts)]
#[instruction(amount: u64, basis_points: u32, duration: u64)]
pub struct InitLoan<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
        constraint = deposit_token_account.amount == 1,
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = borrower,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        space = Loan::space(),
        bump,
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(
        init_if_needed,
        payer = borrower,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        space = TokenManager::space(),
        bump,
    )]   
    pub token_manager_account: Account<'info, TokenManager>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_loan(
  ctx: Context<InitLoan>,
  amount: u64,
  basis_points: u32,
  duration: u64
) -> Result<()> {
  let loan = &mut ctx.accounts.loan_account;

  // Init
  loan.mint = ctx.accounts.mint.key();
  loan.borrower = ctx.accounts.borrower.key();
  loan.bump = *ctx.bumps.get("loan_account").unwrap();
  //
  loan.amount = amount;
  loan.basis_points = basis_points;
  loan.duration = duration;
  loan.state = LoanState::Listed;
  // Delegate authority
  anchor_spl::token::approve(
      CpiContext::new(
          ctx.accounts.token_program.to_account_info(),
          anchor_spl::token::Approve {
              to: ctx.accounts.deposit_token_account.to_account_info(),
              delegate: loan.to_account_info(),
              authority: ctx.accounts.borrower.to_account_info(),
          }
      ),
      1
  )?;

  let signer_bump = &[loan.bump];
  let signer_seeds = &[&[
      Loan::PREFIX,
      loan.mint.as_ref(),
      loan.borrower.as_ref(),
      signer_bump
  ][..]];

  freeze(
      FreezeParams {
          delegate: loan.to_account_info(),
          token_account: ctx.accounts.deposit_token_account.to_account_info(),
          edition: ctx.accounts.edition.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          signer_seeds: signer_seeds
      }
  )?;

  Ok(())
}