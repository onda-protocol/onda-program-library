use anchor_lang::{
  prelude::*,
  solana_program::{
      program::{invoke},
      system_instruction::{transfer}
  }
};
use anchor_spl::token::{Mint};
use crate::utils::*;
use crate::constants::*;
use crate::error::*;
use crate::state::{Loan, LoanState};

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct RepayLoan<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub lender: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        has_one = borrower,
        has_one = mint,
        constraint = loan.lender.unwrap() == lender.key(), 
        constraint = loan.state == LoanState::Active
    )]
    pub loan: Box<Account<'info, Loan>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handle_repay_loan<'info>(ctx: Context<'_, '_, '_, 'info, RepayLoan<'info>>, amount: u64) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let borrower = &ctx.accounts.borrower;
    let mint = &ctx.accounts.mint;

    let payment = std::cmp::min(amount, loan.outstanding);
    let duration = ctx.accounts.clock.unix_timestamp.checked_sub(
        loan.start_date.unwrap()
    ).ok_or(ErrorCodes::NumericalOverflow)?;

    let expiry = loan.start_date.unwrap().checked_add(loan.duration).ok_or(ErrorCodes::NumericalOverflow)?;
    let is_overdue = ctx.accounts.clock.unix_timestamp > expiry;
    
    let interest_due = calculate_loan_repayment_fee(
        payment,
        loan.basis_points,
        duration,
        is_overdue
    )?;
    let amount_due = payment.checked_add(interest_due).ok_or(ErrorCodes::NumericalOverflow)?;

    invoke(
        &transfer(
            &loan.borrower,
            &loan.lender.unwrap(),
            amount_due,
        ),
        &[
            borrower.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ]
    )?;

    let creator_fee = calculate_loan_repayment_fee(
        loan.amount.unwrap(),
        loan.creator_basis_points,
        duration,
        false
    )?;

    pay_creator_fees(
        creator_fee,
        10_000, // 100%
        &mint.to_account_info(),
        &ctx.accounts.metadata.to_account_info(),
        &mut borrower.to_account_info(),
        &mut ctx.remaining_accounts.iter(),
    )?;

    loan.outstanding = loan.outstanding - payment;
    
    msg!("Repaid {}", payment);
    msg!("Amount outstanding: {}", loan.outstanding);
    
    if loan.outstanding == 0 {
        msg!("Loan fully repaid");
        loan.state = LoanState::Repaid;
        msg!("Loan state {:?}", loan.state);
    }

    Ok(())
}