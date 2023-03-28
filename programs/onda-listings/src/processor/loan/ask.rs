use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, Collection, TokenManager};
use crate::utils::*;
use crate::error::*;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(amount: u64, basis_points: u16, duration: u64)]
pub struct AskLoan<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.mint == mint.key(),
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
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
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        init_if_needed,
        payer = borrower,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
        ],
        space = TokenManager::space(),
        bump,
        constraint = token_manager.authority == Some(borrower.key()) || token_manager.authority == None @ ErrorCodes::Unauthorized,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
        constraint = collection.config.loan_enabled == true
    )]
    pub collection: Box<Account<'info, Collection>>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: validated in cpi
    pub token_record: Option<UncheckedAccount<'info>>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules_program: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub authorization_rules: Option<UncheckedAccount<'info>>, 
    /// Misc
    /// CHECK: not supported by anchor? used in cpi
    pub sysvar_instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_ask_loan(
  ctx: Context<AskLoan>,
  amount: u64,
  basis_points: u16,
  duration: i64
) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let borrower = &ctx.accounts.borrower;
    let token_manager = &mut ctx.accounts.token_manager;
    let collection = &ctx.accounts.collection;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let token_record = &ctx.accounts.token_record;
    let mint = &ctx.accounts.mint;
    let metadata = &ctx.accounts.metadata;
    let edition = &ctx.accounts.edition;
    let token_program = &ctx.accounts.token_program;
    let system_program = &ctx.accounts.system_program;
    let sysvar_instructions = &ctx.accounts.sysvar_instructions;
    let authorization_rules_program = &ctx.accounts.authorization_rules_program;
    let authorization_rules = &ctx.accounts.authorization_rules;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    require_eq!(token_manager.accounts.call_option, false, ErrorCodes::InvalidState);

    // Init
    loan.mint = ctx.accounts.mint.key();
    loan.borrower = ctx.accounts.borrower.key();
    loan.bump = *ctx.bumps.get("loan").unwrap();
    //
    Loan::init_ask_state(loan, amount, collection.config.loan_basis_points, basis_points, duration)?;
    //
    token_manager.accounts.loan = true;
    token_manager.authority = Some(borrower.key());
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    // Freeze deposit token account
    if deposit_token_account.delegate.is_some() {
        if deposit_token_account.delegate.unwrap() != token_manager.key() {
            return err!(ErrorCodes::InvalidState);
        }
    } else {
        handle_delegate_and_freeze(
            token_manager,
            borrower.to_account_info(),
            deposit_token_account.to_account_info(),
            match token_record {
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
            }
        )?;
    }

    Ok(())
}