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
            borrower.key().as_ref()
        ],
        bump,
        constraint = token_manager.accounts.rental == false,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        init,
        seeds = [b"escrow_token_account", token_manager.key().as_ref()],
        bump,
        payer = lender,
        token::mint = mint,
        token::authority = token_manager,
    )]
    pub escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub escrow_token_record: Option<UncheckedAccount<'info>>,
    /// CHECK: contrained on loan_account
    pub mint: Box<Account<'info, Mint>>,
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
    let lender_token_account = &mut ctx.accounts.lender_token_account;
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
        token_manager.to_account_info(),
        match escrow_token_record {
            Some(account) => Some(account.to_account_info()),
            None => None,
        }, 
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

#[derive(Accounts)]
pub struct RepossessWithRental<'info> {
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
            Rental::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        close = borrower
    )]
    pub rental: Box<Account<'info, Rental>>,
    /// CHECK: constrained by seeds
    #[account(
        mut,
        seeds = [
            Rental::ESCROW_PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
    )]
    pub rental_escrow: AccountInfo<'info>,  
    #[account(
        mut,
        constraint = token_account.mint == mint.key(),
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref()
        ],
        bump,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    /// CHECK: contrained on loan_account
    pub mint: Account<'info, Mint>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_repossess_with_rental<'info>(ctx: Context<'_, '_, '_, 'info, RepossessWithRental<'info>>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let rental = &mut ctx.accounts.rental;
    let token_manager = &mut ctx.accounts.token_manager;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    let start_date = loan.start_date.unwrap();
    let duration = unix_timestamp - start_date;

    if loan.duration > duration  {
        return err!(ErrorCodes::NotOverdue);
    }

    loan.state = LoanState::Defaulted;
    token_manager.accounts.loan = false;
    token_manager.accounts.rental = false;

    if rental.borrower.is_some() {
        settle_rental_escrow_balance(
            rental,
            &mut ctx.accounts.rental_escrow,
            &ctx.accounts.borrower,
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.metadata.to_account_info(),
            &mut ctx.remaining_accounts.iter(),
            unix_timestamp,
        )?;
    }

    // thaw_and_transfer_from_token_account(
    //     token_manager,
    //     ctx.accounts.token_program.to_account_info(),
    //     ctx.accounts.borrower.to_account_info(),
    //     ctx.accounts.token_account.to_account_info(),
    //     ctx.accounts.lender_token_account.to_account_info(),
    //     ctx.accounts.mint.to_account_info(),
    //     ctx.accounts.edition.to_account_info(),
    // )?;

    Ok(())
}