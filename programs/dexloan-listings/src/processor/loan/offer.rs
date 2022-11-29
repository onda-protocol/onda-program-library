use anchor_lang::{prelude::*};
use crate::state::{LoanOffer, Collection};
use crate::constants::*;

#[derive(Accounts)]
#[instruction(amount: u64, basis_points: u32, duration: u64, id: u8)]
pub struct OfferLoan<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(
        init,
        seeds = [
            LoanOffer::PREFIX,
            collection.mint.as_ref(),
            lender.key().as_ref(),
            &[id],
        ],
        payer = lender,
        space = LoanOffer::space(),
        bump,
    )]
    pub loan_offer: Box<Account<'info, LoanOffer>>,
    /// CHECK: seeds
    #[account(
        init_if_needed,
        seeds=[
            LoanOffer::VAULT_PREFIX,
            loan_offer.key().as_ref()
        ],
        payer = lender,
        space = 0,
        bump,
    )]
    pub escrow_payment_account: UncheckedAccount<'info>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
        constraint = collection.config.loan_enabled == true
    )]
    pub collection: Box<Account<'info, Collection>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_offer_loan(
  ctx: Context<OfferLoan>,
  amount: u64,
  basis_points: u32,
  duration: i64,
  offer_id: u8,
) -> Result<()> {
    let offer = &mut ctx.accounts.loan_offer;

    // Init
    offer.collection = ctx.accounts.collection.key();
    offer.bump = *ctx.bumps.get("loan_offer").unwrap();
    offer.escrow_bump = *ctx.bumps.get("escrow_payment_account").unwrap();
    //
    offer.id = offer_id;
    offer.lender = ctx.accounts.lender.key();
    offer.amount = Some(amount);
    offer.basis_points = basis_points;
    offer.duration = duration;
    offer.ltv = None;
    offer.threshold = None;

    // Transfer amount
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.lender.key(),
            &ctx.accounts.escrow_payment_account.key(),
            offer.amount.unwrap(),
        ),
        &[
            ctx.accounts.lender.to_account_info(),
            ctx.accounts.escrow_payment_account.to_account_info(),
        ]
    )?;

    Ok(())
}