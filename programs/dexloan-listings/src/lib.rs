pub mod processor;
pub mod error;
pub mod state;
pub mod constants;
pub mod utils;

use anchor_lang::prelude::*;
pub use processor::*;
pub use error::*;
pub use state::*;
pub use constants::*;
pub use utils::*;

declare_id!("GDNxgyEcP6b2FtTtCGrGhmoy5AQEiwuv26hV1CLmL1yu");

#[program]
pub mod dexloan_listings {
    use super::*;

    // Loans
    pub fn offer_loan<'info>(
        ctx: Context<'_, '_, '_, 'info, OfferLoan<'info>>,
        amount: u64,
        basis_points: u32,
        duration: i64,
        id: u8
    ) -> Result<()> {
        handle_offer_loan(ctx, amount, basis_points, duration, id)
    }

    pub fn take_loan_offer<'info>(
        ctx: Context<'_, '_, '_, 'info, TakeLoanOffer<'info>>,
        id: u8,
    ) -> Result<()> {
        handle_take_loan_offer(ctx, id)
    }

    pub fn close_loan_offer<'info>(
        ctx: Context<'_, '_, '_, 'info, CloseLoanOffer<'info>>,
        id: u8,
    ) -> Result<()> {
        handle_close_loan_offer(ctx, id)
    }

    pub fn ask_loan<'info>(
        ctx: Context<'_, '_, '_, 'info, AskLoan<'info>>,
        amount: u64,
        basis_points: u32,
        duration: i64
    ) -> Result<()> {
        handle_ask_loan(ctx, amount, basis_points, duration)
    }

    pub fn give_loan<'info>(ctx: Context<'_, '_, '_, 'info, GiveLoan<'info>>) -> Result<()> {
        handle_give_loan(ctx)
    }

    pub fn close_loan<'info>(ctx: Context<'_, '_, '_, 'info, CloseLoan<'info>>) -> Result<()> {
        handle_close_loan(ctx)
    }

    pub fn repay_loan<'info>(ctx: Context<'_, '_, '_, 'info, RepayLoan<'info>>) -> Result<()> {
        handle_repay_loan(ctx)
    }

    pub fn repossess<'info>(ctx: Context<'_, '_, '_, 'info, Repossess<'info>>) -> Result<()> {
        handle_repossess(ctx)
    }

    pub fn repossess_with_hire<'info>(ctx: Context<'_, '_, '_, 'info, RepossessWithHire<'info>>) -> Result<()> {
        handle_repossess_with_hire(ctx)
    }

    // Call Options
    pub fn bid_call_option(
        ctx: Context<BidCallOption>,
        amount: u64,
        strike_price: u64,
        expiry: i64,
        id: u8,
    ) -> Result<()> {
        handle_bid_call_option(ctx, amount, strike_price, expiry, id)
    }

    pub fn close_call_option_bid(ctx: Context<CloseCallOptionBid>, id: u8) -> Result<()> {
        handle_close_call_option_bid(ctx, id)
    }

    pub fn sell_call_option<'info>(
        ctx: Context<'_, '_, '_, 'info, SellCallOption<'info>>,
        id: u8,
    ) -> Result<()> {
        handle_sell_call_option(ctx, id)
    }

    pub fn ask_call_option<'info>(
        ctx: Context<'_, '_, '_, 'info, AskCallOption<'info>>,
        amount: u64,
        strike_price: u64,
        expiry: i64,
    ) -> Result<()> {
        handle_ask_call_option(ctx, amount, strike_price, expiry)
    }

    pub fn buy_call_option<'info>(ctx: Context<'_, '_, '_, 'info, BuyCallOption<'info>>) -> Result<()> {
        handle_buy_call_option(ctx)
    }

    pub fn exercise_call_option<'info>(ctx: Context<'_, '_, '_, 'info, ExerciseCallOption<'info>>) -> Result<()> {
        handle_exercise_call_option(ctx)
    }

    pub fn exercise_call_option_with_hire<'info>(ctx: Context<'_, '_, '_, 'info, ExerciseCallOptionWithHire<'info>>) -> Result<()> {
        handle_exercise_call_option_with_hire(ctx)
    }

    pub fn close_call_option<'info>(ctx: Context<'_, '_, '_, 'info, CloseCallOption<'info>>) -> Result<()> {
        handle_close_call_option(ctx)
    }

    // Hires
    pub fn init_hire<'info>(
        ctx: Context<'_, '_, '_, 'info, InitHire<'info>>,
        args: HireArgs
    ) -> Result<()> {
        handle_init_hire(ctx, args)
    }

    pub fn take_hire<'info>(ctx: Context<'_, '_, '_, 'info, TakeHire<'info>>, days: u16) -> Result<()> {
        handle_take_hire(ctx, days)
    }

    pub fn extend_hire<'info>(ctx: Context<'_, '_, '_, 'info, ExtendHire<'info>>, days: u16) -> Result<()> {
        handle_extend_hire(ctx, days)
    }

    pub fn recover_hire<'info>(ctx: Context<'_, '_, '_, 'info, RecoverHire<'info>>) -> Result<()> {
        handle_recover_hire(ctx)
    }

    pub fn withdraw_from_hire_escrow<'info>(ctx: Context<'_, '_, '_, 'info, WithdrawFromHireEscrow<'info>>) -> Result<()> {
        handle_withdraw_from_hire_escrow(ctx)
    }

    pub fn close_hire<'info>(ctx: Context<'_, '_, '_, 'info, CloseHire<'info>>) -> Result<()> {
        handle_close_hire(ctx)
    }

    // Collection
    pub fn init_collection(ctx: Context<InitCollection>, config: Config) -> Result<()> {
        handle_init_collection(ctx, config)
    }

    pub fn close_collection(ctx: Context<CloseCollection>) -> Result<()> {
        handle_close_collection(ctx)
    }
}