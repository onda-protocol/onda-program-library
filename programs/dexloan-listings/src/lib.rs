use anchor_lang::prelude::*;

mod instructions;
use instructions::{call_option::*, loan::*};

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
        instructions::loan::init_loan(ctx, amount, basis_points, duration)
    }

    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        instructions::loan::cancel_listing(ctx)
    }

    pub fn make_loan(ctx: Context<MakeLoan>) -> Result<()> {
        instructions::loan::make_loan(ctx)
    }

    pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
        instructions::loan::repay_loan(ctx)
    }

    pub fn repossess_collateral(ctx: Context<RepossessCollateral>) -> Result<()> {
        instructions::loan::repossess_collateral(ctx)
    }

    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        instructions::loan::close_account(ctx)
    }

    pub fn init_call_option(
        ctx: Context<InitCallOption>,
        amount: u64,
        strike_price: u64,
        expiry: i64
    ) -> Result<()> {
        instructions::call_option::init_call_option(ctx, amount, strike_price, expiry)
    }

    pub fn buy_call_option(ctx: Context<BuyCallOption>) -> Result<()> {
        instructions::call_option::buy_call_option(ctx)
    }

    pub fn exercise_call_option(ctx: Context<ExerciseOption>) -> Result<()> {
        instructions::call_option::exercise_call_option(ctx)
    }

    pub fn close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
        instructions::call_option::close_call_option(ctx)
    }
}