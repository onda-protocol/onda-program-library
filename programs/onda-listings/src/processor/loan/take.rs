use {
    anchor_lang::{
        prelude::*,
        solana_program::{
            program::{invoke_signed},
            system_instruction::{transfer}
        },
    },
    anchor_spl::token::{Mint, Token, TokenAccount}
};
use crate::state::{Loan, LoanOffer, Collection, TokenManager};
use crate::utils::*;
use crate::error::*;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct TakeLoanOffer<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(mut)]
    /// CHECK: seeds
    pub lender: AccountInfo<'info>,
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
        mut,
        seeds = [
            LoanOffer::PREFIX,
            collection.mint.as_ref(),
            lender.key().as_ref(),
            &[id],
        ],
        close = lender,
        bump,
    )]
    pub loan_offer: Box<Account<'info, LoanOffer>>,
    #[account(
        mut,
        seeds=[
            LoanOffer::VAULT_PREFIX,
            loan_offer.key().as_ref()
        ],
        bump,
    )]
    /// CHECK: seeds
    pub escrow_payment_account: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = borrower,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref()
        ],
        space = TokenManager::space(),
        bump,
    )]
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
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
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_take_loan_offer(
  ctx: Context<TakeLoanOffer>,
  _id: u8,
) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let offer = &mut ctx.accounts.loan_offer;
    let borrower = &mut ctx.accounts.borrower;
    let lender = &mut ctx.accounts.lender;
    let escrow_payment_account = &mut ctx.accounts.escrow_payment_account;
    let token_manager = &mut ctx.accounts.token_manager;
    let mint = &ctx.accounts.mint;
    let metadata = &ctx.accounts.metadata;
    let edition = &ctx.accounts.edition;
    let collection = &ctx.accounts.collection;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let token_record = &ctx.accounts.token_record;
    let token_program = &ctx.accounts.token_program;
    let system_program = &ctx.accounts.system_program;
    let sysvar_instructions = &ctx.accounts.sysvar_instructions;
    let authorization_rules_program = &ctx.accounts.authorization_rules_program;
    let authorization_rules = &ctx.accounts.authorization_rules;

    assert_collection_valid(
        &metadata,
        mint.key(),
        collection.key(),
        ctx.program_id.clone(),
    )?;

    require_eq!(token_manager.accounts.call_option, false, ErrorCodes::InvalidState);

    // Init
    loan.mint = mint.key();
    loan.borrower = borrower.key();
    loan.lender = Some(lender.key());
    loan.bump = *ctx.bumps.get("loan").unwrap();
    //
    Loan::init_ask_state(
        loan,
        offer.amount.unwrap(),
        collection.config.loan_basis_points,
        offer.basis_points,
        offer.duration
    )?;
    Loan::set_active(loan, ctx.accounts.clock.unix_timestamp)?;
    //
    token_manager.authority = Some(loan.borrower);
    token_manager.accounts.loan = true;
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    handle_delegate_and_freeze(
        token_manager,
        borrower.to_account_info(),
        deposit_token_account.to_account_info(),
        if let Some(token_record) = token_record {
            Some(token_record.to_account_info())
        } else {
            None
        },
        mint.to_account_info(),
        metadata.to_account_info(),
        edition.to_account_info(),
        token_program.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        authorization_rules_program.to_account_info(),
        if let Some(authorization_rules) = authorization_rules {
            Some(authorization_rules.to_account_info())
        } else {
            None
        },
    )?;

    // Transfer loan amount from offer escrow
    let loan_offer_pubkey = offer.key();
    let signer_bump = &[offer.escrow_bump];
    let signer_seeds = &[&[
        LoanOffer::VAULT_PREFIX,
        loan_offer_pubkey.as_ref(),
        signer_bump
    ][..]];

    invoke_signed(
        &transfer(
            &escrow_payment_account.key(),
            &loan.borrower,
            offer.amount.unwrap(),
        ),
        &[
            escrow_payment_account.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}