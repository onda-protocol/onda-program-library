use anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke, invoke_signed},
    }
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use mpl_token_metadata::{
    instruction::{freeze_delegated_account, thaw_delegated_account}
};
use crate::state::{Loan, LoanState};
use crate::error::{ErrorCode};

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

pub const SECONDS_PER_YEAR: f64 = 31_536_000.0; 

pub fn init(
    ctx: Context<InitLoan>,
    amount: u64,
    basis_points: u32,
    duration: u64
) -> Result<()> {
    let loan = &mut ctx.accounts.loan_account;
    let loan_bump = ctx.bumps.get("loan_account").unwrap().clone(); // TODO unwrap_or

    // Init
    loan.mint = ctx.accounts.mint.key();
    loan.bump = loan_bump;
    //
    loan.amount = amount;
    loan.basis_points = basis_points;
    loan.duration = duration;
    loan.state = LoanState::Listed;
    loan.borrower = ctx.accounts.borrower.key();
    // Transfer
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Approve {
        to: ctx.accounts.deposit_token_account.to_account_info(),
        delegate: loan.to_account_info(),
        authority: ctx.accounts.borrower.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    anchor_spl::token::approve(cpi_ctx, 1)?;

    let account_infos = &[
        loan.to_account_info(),
        ctx.accounts.deposit_token_account.to_account_info(),
        ctx.accounts.edition.to_account_info(),
        ctx.accounts.mint.to_account_info()
    ];
    let freeze_instruction = &freeze_delegated_account(
        mpl_token_metadata::ID,
        loan.key(),
        ctx.accounts.deposit_token_account.key(),
        ctx.accounts.edition.key(),
        ctx.accounts.mint.key()
    );
    let bump_seed = &[loan_bump];
    let signer_seeds = &[&[
        Loan::PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        ctx.accounts.borrower.to_account_info().key.as_ref(),
        bump_seed
    ][..]];

    invoke_signed(
        freeze_instruction,
        account_infos,
        signer_seeds
    )?;

    Ok(())
}

pub fn close(ctx: Context<CloseLoan>) -> Result<()> {
    // TODO check if frozen
    thaw(
        ctx.accounts.loan_account.bump,
        ThawParams {
            loan: ctx.accounts.loan_account.to_account_info(),
            borrower: ctx.accounts.borrower.to_account_info(),
            deposit_token_account: ctx.accounts.deposit_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        }
    )?;

    anchor_spl::token::revoke(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Revoke {
                source: ctx.accounts.deposit_token_account.to_account_info(),
                authority: ctx.accounts.borrower.to_account_info(),
            }
        )
    )?;
    
    Ok(())
}

pub fn lend(ctx: Context<Lend>) -> Result<()> {
    let listing = &mut ctx.accounts.loan_account;

    listing.state = LoanState::Active;
    listing.lender = ctx.accounts.lender.key();
    listing.start_date = ctx.accounts.clock.unix_timestamp;
    // Transfer amount
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &listing.lender,
            &listing.borrower,
            listing.amount,
        ),
        &[
            ctx.accounts.lender.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
        ]
    )?;

    Ok(())
}

pub fn repay(ctx: Context<RepayLoan>) -> Result<()> {
    let loan = &mut ctx.accounts.loan_account;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp;
    let loan_start_date = loan.start_date;
    let loan_basis_points = loan.basis_points as f64;
    let loan_duration = (unix_timestamp - loan_start_date) as f64;
    let pro_rata_interest_rate = ((loan_basis_points / 10_000 as f64) / SECONDS_PER_YEAR) * loan_duration;
    let interest_due = loan.amount as f64 * pro_rata_interest_rate;
    let amount_due = loan.amount + interest_due.round() as u64;
    
    msg!("Loan basis points: {}", loan_basis_points);
    msg!("Loan duration: {} seconds", loan_duration);
    msg!("Loan amount: {} LAMPORTS", loan.amount);
    msg!("Pro Rata interest rate: {}%", pro_rata_interest_rate);
    msg!("Interest due: {} LAMPORTS", interest_due);
    msg!("Total amount due: {} LAMPORTS", amount_due);

    // Transfer payment
    invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &loan.borrower,
            &loan.lender,
            amount_due as u64,
        ),
        &[
            ctx.accounts.borrower.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ]
    )?;

    thaw(
        loan.bump,
        ThawParams {
            loan: ctx.accounts.loan_account.to_account_info(),
            borrower: ctx.accounts.borrower.to_account_info(),
            deposit_token_account: ctx.accounts.deposit_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        }
    )?;

    let cpi_accounts = anchor_spl::token::Revoke {
        source: ctx.accounts.deposit_token_account.to_account_info(),
        authority: ctx.accounts.borrower.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    anchor_spl::token::revoke(cpi_ctx)?;

    Ok(())
}

pub fn repossess(ctx: Context<Repossess>) -> Result<()> {
    let loan = &mut ctx.accounts.loan_account;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp as u64;
    let loan_start_date = loan.start_date as u64;
    let loan_duration = unix_timestamp - loan_start_date;

    msg!("Loan start date: {} seconds", loan_start_date);
    msg!("Loan duration: {} seconds", loan.duration);
    msg!("Time passed: {} seconds", loan_duration);

    if loan.duration > loan_duration  {
        return Err(ErrorCode::NotOverdue.into())
    }
    
    loan.state = LoanState::Defaulted;

    thaw(
        loan.bump,
        ThawParams {
            loan: loan.to_account_info(),
            borrower: ctx.accounts.borrower.to_account_info(),
            deposit_token_account: ctx.accounts.deposit_token_account.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        }
    )?;

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.deposit_token_account.to_account_info(),
        to: ctx.accounts.lender_token_account.to_account_info(),
        authority: loan.to_account_info(),
    };
    let seeds = &[
        Loan::PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        ctx.accounts.borrower.to_account_info().key.as_ref(),
        &[loan.bump],
    ];
    let signer = &[&seeds[..]];
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
        1
    )?;

    let revoke_ix = spl_token::instruction::revoke(
        &spl_token::ID,
        ctx.accounts.deposit_token_account.to_account_info().key,
        ctx.accounts.borrower.key,
        &[loan.to_account_info().key],
    )?;
    let signer_bump = &[loan.bump];
    let signer_seeds = &[&[
        Loan::PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        ctx.accounts.borrower.key.as_ref(),
        signer_bump,
    ][..]];

    invoke_signed(
        &revoke_ix,
        &[
            loan.to_account_info(),
            ctx.accounts.deposit_token_account.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
        ],
        signer_seeds
    )?;

    Ok(())
}

pub struct ThawParams<'a> {
    /// CHECK
    loan: AccountInfo<'a>,
    /// CHECK
    borrower: AccountInfo<'a>,
    /// CHECK
    deposit_token_account: AccountInfo<'a>,
    /// CHECK
    edition: AccountInfo<'a>,
    /// CHECK
    mint: AccountInfo<'a>,
}

pub fn thaw<'a>(bump: u8, params: ThawParams<'a>) -> Result<()> {
    let ThawParams {
        loan,
        borrower,
        deposit_token_account,
        edition,
        mint,
    } = params;
    
    let signer_bump = &[bump];
    let signer_seeds = &[&[
        Loan::PREFIX,
        mint.key.as_ref(),
        borrower.key.as_ref(),
        signer_bump
    ][..]];

    invoke_signed(
        &thaw_delegated_account(
            mpl_token_metadata::ID,
            loan.key(),
            deposit_token_account.key(),
            edition.key(),
            mint.key()
        ),
        &[
            loan,
            deposit_token_account.clone(),
            edition,
            mint
        ],
        signer_seeds
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, basis_points: u32, duration: u64)]
pub struct InitLoan<'info> {
    /// The person who is listing the loan
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.mint == mint.key(),
        constraint = deposit_token_account.owner == borrower.key(),
        constraint = deposit_token_account.amount == 1
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    /// The new listing account
    #[account(
        init,
        payer = borrower,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        space = Loan::space(),
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CloseLoan<'info> {
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = loan_account.bump,
        constraint = loan_account.borrower == *borrower.key,
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed || loan_account.state == LoanState::Defaulted,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Lend<'info> {
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            Loan::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.borrower != lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed,
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
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
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Repossess<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,
    /// CHECK: contrained on loan_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub lender_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        constraint = loan_account.borrower == borrower.key()
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}
