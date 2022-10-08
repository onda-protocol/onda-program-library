use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token};
use crate::state::{CallOptionBid, Collection};
use crate::constants::*;

#[derive(Accounts)]
#[instruction(amount: u64, strike_price: u64, expiry: i64, id: u8)]
pub struct BidCallOption<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        init,
        seeds = [
            CallOptionBid::PREFIX,
            collection.mint.as_ref(),
            buyer.key().as_ref(),
            &[id],
        ],
        payer = buyer,
        space = CallOptionBid::space(),
        bump,
    )]
    pub call_option_bid: Box<Account<'info, CallOptionBid>>,
    #[account(
        init_if_needed,
        seeds=[
            CallOptionBid::VAULT_PREFIX,
            call_option_bid.key().as_ref()
        ],
        payer = buyer,
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

pub fn handle_bid_call_option(
  ctx: Context<BidCallOption>,
  amount: u64,
  strike_price: u64,
  expiry: i64,
  offer_id: u8,
) -> Result<()> {
    let bid = &mut ctx.accounts.call_option_bid;

    // Init
    bid.collection = ctx.accounts.collection.key();
    bid.bump = *ctx.bumps.get("call_option_bid").unwrap();
    bid.escrow_bump = *ctx.bumps.get("escrow_payment_account").unwrap();
    //
    bid.id = offer_id;
    bid.buyer = ctx.accounts.buyer.key();
    bid.amount = amount;
    bid.strike_price = strike_price;
    bid.expiry = expiry;

    // Transfer amount
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.escrow_payment_account.key(),
            bid.amount,
        ),
        &[
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.escrow_payment_account.to_account_info(),
        ]
    )?;

    Ok(())
}