use anchor_lang::prelude::*;

mod instructions;
use instructions::{call_option::*, loan::*, listing::*};

pub mod error;
pub mod state;

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

#[program]
pub mod dexloan_listings {
    use super::*;

    pub fn init_loan(
        ctx: Context<InitLoan>,
        amount: u64,
        basis_points: u32,
        duration: u64
    ) -> Result<()> {
        instructions::loan::init(ctx, amount, basis_points, duration)
    }

    pub fn cancel_listing(ctx: Context<CloseLoan>) -> Result<()> {
        instructions::loan::close(ctx)
    }

    pub fn make_loan(ctx: Context<Lend>) -> Result<()> {
        instructions::loan::lend(ctx)
    }

    pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
        instructions::loan::repay(ctx)
    }

    pub fn repossess_collateral(ctx: Context<Repossess>) -> Result<()> {
        instructions::loan::repossess(ctx)
    }

    pub fn init_option(
        ctx: Context<InitCallOption>,
        amount: u64,
        strike_price: u64,
        expiry: i64
    ) -> Result<()> {
        instructions::call_option::init(ctx, amount, strike_price, expiry)
    }

    pub fn buy_call_option(ctx: Context<BuyCallOption>) -> Result<()> {
        instructions::call_option::buy(ctx)
    }

    pub fn exercise_call_option(ctx: Context<ExerciseCallOption>) -> Result<()> {
        instructions::call_option::exercise(ctx)
    }

    pub fn close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
        instructions::call_option::close(ctx)
    }

    pub fn migrate_listing(ctx: Context<CloseListing>) -> Result<()> {
        instructions::listing::close(ctx)
    }

    pub fn close_listing(ctx: Context<CloseListing>) -> Result<()> {
        instructions::listing::close(ctx)
    }
}