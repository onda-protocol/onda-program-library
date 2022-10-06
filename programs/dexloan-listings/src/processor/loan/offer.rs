use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
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
    #[account(
        init,
        seeds=[
            LoanOffer::VAULT_PREFIX,
            collection.mint.as_ref(),
            lender.key().as_ref(),
            &[id],
        ],
        payer = lender,
        space = 0,
        bump,
    )]
    pub loan_offer_sol_vault: UncheckedAccount<'info>,
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
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
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
    offer.bump = *ctx.bumps.get("offer").unwrap();
    //
    offer.id = offer_id;
    offer.amount = Some(amount);
    offer.basis_points = basis_points;
    offer.duration = duration;
    offer.ltv = None;
    offer.threshold = None;

    // Transfer amount
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.lender.key(),
            &ctx.accounts.loan_offer_sol_vault.key(),
            offer.amount.unwrap(),
        ),
        &[
            ctx.accounts.lender.to_account_info(),
            ctx.accounts.loan_offer_sol_vault.to_account_info(),
        ]
    )?;

    Ok(())
}