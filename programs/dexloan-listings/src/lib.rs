use anchor_lang::prelude::*;

mod instructions;
use instructions::{call_option::*, loan::*, hire::*, listing::*};

pub mod error;
pub mod state;
pub mod utils;

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

#[program]
pub mod dexloan_listings {
    use super::*;

    // Loans
    pub fn init_loan<'info>(
        ctx: Context<'_, '_, '_, 'info, InitLoan<'info>>,
        amount: u64,
        basis_points: u32,
        duration: u64
    ) -> Result<()> {
        instructions::loan::init(ctx, amount, basis_points, duration)
    }

    pub fn close_loan<'info>(ctx: Context<'_, '_, '_, 'info, CloseLoan<'info>>) -> Result<()> {
        instructions::loan::close(ctx)
    }

    pub fn give_loan<'info>(ctx: Context<'_, '_, '_, 'info, Lend<'info>>) -> Result<()> {
        instructions::loan::lend(ctx)
    }

    pub fn repay_loan<'info>(ctx: Context<'_, '_, '_, 'info, RepayLoan<'info>>) -> Result<()> {
        instructions::loan::repay(ctx)
    }

    pub fn repossess_collateral<'info>(ctx: Context<'_, '_, '_, 'info, Repossess<'info>>) -> Result<()> {
        instructions::loan::repossess(ctx)
    }

    // Call Options
    pub fn init_call_option(
        ctx: Context<InitCallOption>,
        amount: u64,
        strike_price: u64,
        expiry: i64
    ) -> Result<()> {
        instructions::call_option::init(ctx, amount, strike_price, expiry)
    }

    pub fn buy_call_option<'info>(ctx: Context<'_, '_, '_, 'info, BuyCallOption<'info>>) -> Result<()> {
        instructions::call_option::buy(ctx)
    }

    pub fn exercise_call_option<'info>(ctx: Context<'_, '_, '_, 'info, ExerciseCallOption<'info>>) -> Result<()> {
        instructions::call_option::exercise(ctx)
    }

    pub fn close_call_option<'info>(ctx: Context<'_, '_, '_, 'info, CloseCallOption<'info>>) -> Result<()> {
        instructions::call_option::close(ctx)
    }

    // Hires
    pub fn init_hire<'info>(
        ctx: Context<'_, '_, '_, 'info, InitHire<'info>>,
        amount: u64,
        expiry: i64,
        borrower: Option<Pubkey>
    ) -> Result<()> {
        instructions::hire::init(ctx, amount, expiry, borrower)
    }

    pub fn take_hire<'info>(ctx: Context<'_, '_, '_, 'info, TakeHire<'info>>) -> Result<()> {
        instructions::hire::take(ctx)
    }

    pub fn revoke_hire<'info>(ctx: Context<'_, '_, '_, 'info, RevokeHire<'info>>) -> Result<()> {
        instructions::hire::revoke(ctx)
    }

    pub fn close_hire<'info>(ctx: Context<'_, '_, '_, 'info, CloseHire<'info>>) -> Result<()> {
        instructions::hire::close(ctx)
    }

    // Deprecated v1 Listings
    pub fn close_listing<'info>(ctx: Context<'_, '_, '_, 'info, CloseListing<'info>>) -> Result<()> {
        instructions::listing::close(ctx)
    }

    pub fn cancel_listing<'info>(ctx: Context<'_, '_, '_, 'info, CancelListing<'info>>) -> Result<()> {
        instructions::listing::cancel_listing(ctx)
    }
}