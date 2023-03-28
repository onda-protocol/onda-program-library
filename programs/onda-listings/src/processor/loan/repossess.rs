use {
    anchor_lang::{prelude::*},
    anchor_spl::{
        associated_token::{AssociatedToken},
        token::{Token, TokenAccount, Mint}
    }
};
use crate::state::{Loan, LoanState, Rental, TokenManager};
use crate::error::{ErrorCodes};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct Repossess<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender
    )]
    pub lender_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub lender_token_record: Option<UncheckedAccount<'info>>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub deposit_token_record: Option<UncheckedAccount<'info>>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        has_one = borrower,
        has_one = mint,
        constraint = loan.lender.unwrap() == lender.key(), 
        constraint = loan.state == LoanState::Active,
    )]
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
        ],
        bump,
        constraint = token_manager.accounts.rental == false,
        constraint = token_manager.authority == Some(borrower.key()) @ ErrorCodes::Unauthorized,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        init,
        seeds = [
            TokenManager::ESCROW_PREFIX,
            token_manager.key().as_ref(),
        ],
        bump,
        payer = lender,
        token::mint = mint,
        token::authority = token_manager,
    )]
    pub escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: contrained on loan_account
    pub escrow_token_record: Option<UncheckedAccount<'info>>,
    /// CHECK: contrained on loan_account
    pub mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules: Option<UncheckedAccount<'info>>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: not supported by anchor? used in cpi
    pub sysvar_instructions: UncheckedAccount<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_repossess(ctx: Context<Repossess>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let token_manager = &mut ctx.accounts.token_manager;
    let borrower = &mut ctx.accounts.borrower;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let deposit_token_record = &mut ctx.accounts.deposit_token_record;
    let lender = &mut ctx.accounts.lender;
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

  
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;
    let start_date = loan.start_date.unwrap();
    let duration = unix_timestamp - start_date;

    if loan.duration > duration  {
        return err!(ErrorCodes::NotOverdue)
    }

    handle_thaw_and_transfer(
        token_manager,
        borrower.to_account_info(),
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
        lender.to_account_info(),
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

    loan.state = LoanState::Defaulted;
    token_manager.accounts.loan = false; 

    Ok(())
}

