use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Hire, HireState, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct HireArgs {
    amount: u64,
    expiry: i64,
    borrower: Option<Pubkey>,
}

#[derive(Accounts)]
pub struct InitHire<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = lender,
        seeds = [
        Hire::PREFIX,
        mint.key().as_ref(),
        lender.key().as_ref(),
        ],
        space = Hire::space(),
        bump,
    )]
    pub hire_account: Account<'info, Hire>,    
    #[account(
        init_if_needed,
        payer = lender,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref()
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
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_hire(
  ctx: Context<InitHire>,
  args: HireArgs,
) -> Result<()> {
    let hire = &mut ctx.accounts.hire_account;
    let token_manager = &mut ctx.accounts.token_manager_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    if unix_timestamp > args.expiry {
        return err!(DexloanError::InvalidExpiry)
    }

    if args.amount == 0 && args.borrower.is_none() {
        return err!(DexloanError::BorrowerNotSpecified)
    }

    // Init
    hire.lender = ctx.accounts.lender.key();
    hire.mint = ctx.accounts.mint.key();
    hire.bump = *ctx.bumps.get("hire_account").unwrap();
    //
    hire.amount = args.amount;
    hire.expiry = args.expiry;
    hire.state = HireState::Listed;
    //
    if args.borrower.is_some() {
        hire.borrower = args.borrower;
    }
    //
    token_manager.accounts.hire = true;

    delegate_and_freeze_token_account(
        token_manager,
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.lender.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.mint.to_account_info()
    )?;

    Ok(())
}