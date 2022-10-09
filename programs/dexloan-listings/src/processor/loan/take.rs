use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
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
            mint.key().as_ref(),
            borrower.key().as_ref()
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
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_take_loan_offer(
  ctx: Context<TakeLoanOffer>,
) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let offer = &mut ctx.accounts.loan_offer;
    let escrow_payment_account = &mut ctx.accounts.escrow_payment_account;
    let token_manager = &mut ctx.accounts.token_manager;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    require_eq!(token_manager.accounts.call_option, false, DexloanError::InvalidState);

    // Init
    loan.mint = ctx.accounts.mint.key();
    loan.borrower = ctx.accounts.borrower.key();
    loan.lender = Some(ctx.accounts.lender.key());
    loan.bump = *ctx.bumps.get("loan").unwrap();
    //
    Loan::init_ask_state(loan, offer.amount.unwrap(), offer.basis_points, offer.duration)?;
    Loan::set_active(loan, ctx.accounts.clock.unix_timestamp)?;
    //
    token_manager.accounts.loan = true;
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    maybe_delegate_and_freeze_token_account(
        token_manager,
        deposit_token_account,
        ctx.accounts.borrower.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.borrower.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    let bump = &[offer.escrow_bump];
    let offer_pubkey = offer.key();
    let signer_seeds = &[&[
        LoanOffer::VAULT_PREFIX,
        offer_pubkey.as_ref(),
        bump
    ][..]];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::system_instruction::transfer(
            &escrow_payment_account.key(),
            &loan.borrower,
            loan.amount.unwrap(),
        ),
        &[
            escrow_payment_account.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}