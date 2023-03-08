use {
    anchor_lang::{
        prelude::*,
        solana_program::{
            program::{invoke_signed},
            system_instruction::{transfer}
        },
        AccountsClose
    },
    anchor_spl::token::{Mint, Token, TokenAccount}
};
use crate::state::{Collection, Loan, LoanState, LoanOffer, TokenManager};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct CloseLoan<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        has_one = mint,
        has_one = borrower,
        constraint = loan.state == LoanState::Listed || loan.state == LoanState::Defaulted,
        close = borrower,
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
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}



pub fn handle_close_loan(ctx: Context<CloseLoan>) -> Result<()> {
    let borrower = &ctx.accounts.borrower;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let token_manager = &mut ctx.accounts.token_manager;

    process_close_loan(
        token_manager,
        borrower,
        deposit_token_account,
        &ctx.accounts.mint,
        &ctx.accounts.edition,
        &ctx.accounts.token_program,
    )?;
  
    Ok(())
}

pub fn process_close_loan<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    borrower: &Signer<'info>,
    deposit_token_account: &Account<'info, TokenAccount>,
    mint: &Account<'info, Mint>,
    edition: &UncheckedAccount<'info>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    token_manager.accounts.loan = false;
    // IMPORTANT CHECK!
    if token_manager.accounts.rental == false {
        if deposit_token_account.is_frozen() {
            thaw_and_revoke_token_account(
                token_manager,
                token_program.to_account_info(),
                deposit_token_account.to_account_info(),
                borrower.to_account_info(),
                mint.to_account_info(),
                edition.to_account_info()
            )?;
        } else if deposit_token_account.delegate.is_some() {
            anchor_spl::token::revoke(
                CpiContext::new(
                    token_program.to_account_info(),
                    anchor_spl::token::Revoke {
                        source: deposit_token_account.to_account_info(),
                        authority: borrower.to_account_info(),
                    }
                )
            )?;
        }
    
        token_manager.close(borrower.to_account_info())?;    
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct CloseLoanOffer<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
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
    /// CHECK: seeds
    #[account(
        mut,
        seeds=[
            LoanOffer::VAULT_PREFIX,
            loan_offer.key().as_ref()
        ],
        bump,
    )]
    pub escrow_payment_account: AccountInfo<'info>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
    )]
    pub collection: Box<Account<'info, Collection>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_close_loan_offer(ctx: Context<CloseLoanOffer>, _id: u8) -> Result<()> {
    let loan_offer = &ctx.accounts.loan_offer;
    let escrow_payment_account = &ctx.accounts.escrow_payment_account;

    let loan_offer_pubkey = loan_offer.key();
    let signer_bump = &[loan_offer.escrow_bump];
    let signer_seeds = &[&[
        LoanOffer::VAULT_PREFIX,
        loan_offer_pubkey.as_ref(),
        signer_bump
    ][..]];

    invoke_signed(
        &transfer(
            &escrow_payment_account.key(),
            &loan_offer.lender,
            loan_offer.amount.unwrap(),
        ),
        &[
            escrow_payment_account.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}