use anchor_lang::{
    prelude::*,
    solana_program::program_option::{COption}
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Loan, LoanState};
use crate::error::{ErrorCode};

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

pub const ESCROW_PREFIX: &'static [u8] = b"escrow";
pub const SECONDS_PER_YEAR: f64 = 31_536_000.0; 

pub fn init(
    ctx: Context<InitLoan>,
    amount: u64,
    basis_points: u32,
    duration: u64
) -> Result<()> {
    let listing = &mut ctx.accounts.loan_account;
    // Init
    listing.mint = ctx.accounts.mint.key();
    listing.escrow = ctx.accounts.escrow_account.key();
    listing.bump = *ctx.bumps.get("loan_account").unwrap();
    listing.escrow_bump = *ctx.bumps.get("escrow_account").unwrap();
    //
    listing.amount = amount;
    listing.basis_points = basis_points;
    listing.duration = duration;
    listing.state = LoanState::Listed;
    listing.borrower = ctx.accounts.borrower.key();
    // Transfer
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Approve {
        to: ctx.accounts.deposit_token_account.to_account_info(),
        delegate: ctx.accounts.escrow_account.to_account_info(),
        authority: ctx.accounts.borrower.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    anchor_spl::token::approve(cpi_ctx, 1)?;

    Ok(())
}

pub fn close(ctx: Context<CloseLoan>) -> Result<()> {
    let listing = &ctx.accounts.loan_account;
    let borrower = &ctx.accounts.borrower;
    let escrow_account = &ctx.accounts.escrow_account;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let token_program = &ctx.accounts.token_program;

    if escrow_account.amount == 0 {
        if deposit_token_account.delegate == COption::Some(escrow_account.key()) {
            let cpi_program = token_program.to_account_info();
            let cpi_accounts = anchor_spl::token::Revoke {
                source: deposit_token_account.to_account_info(),
                authority: borrower.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            anchor_spl::token::revoke(cpi_ctx)?;
        }
    } else {
        let cpi_program = token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: escrow_account.to_account_info(),
            to: deposit_token_account.to_account_info(),
            authority: escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[listing.escrow_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
    }
    
    Ok(())
}

pub fn lend(ctx: Context<Lend>) -> Result<()> {
    let listing = &mut ctx.accounts.loan_account;

    listing.state = LoanState::Active;
    listing.lender = ctx.accounts.lender.key();
    listing.start_date = ctx.accounts.clock.unix_timestamp;
    // Transfer token to escrow
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.deposit_token_account.to_account_info(),
        to: ctx.accounts.escrow_account.to_account_info(),
        authority: ctx.accounts.escrow_account.to_account_info(),
    };
    let seeds = &[
        ESCROW_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[listing.escrow_bump],
    ];
    let signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, 1)?;
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
    let listing = &mut ctx.accounts.loan_account;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp;
    let loan_start_date = listing.start_date;
    let loan_basis_points = listing.basis_points as f64;
    let loan_duration = (unix_timestamp - loan_start_date) as f64;
    let pro_rata_interest_rate = ((loan_basis_points / 10_000 as f64) / SECONDS_PER_YEAR) * loan_duration;
    let interest_due = listing.amount as f64 * pro_rata_interest_rate;
    let amount_due = listing.amount + interest_due.round() as u64;
    
    msg!("Loan basis points: {}", loan_basis_points);
    msg!("Loan duration: {} seconds", loan_duration);
    msg!("Loan amount: {} LAMPORTS", listing.amount);
    msg!("Pro Rata interest rate: {}%", pro_rata_interest_rate);
    msg!("Interest due: {} LAMPORTS", interest_due);
    msg!("Total amount due: {} LAMPORTS", amount_due);

    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &listing.borrower,
            &listing.lender,
            amount_due as u64,
        ),
        &[
            ctx.accounts.borrower.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        ]
    )?;

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.escrow_account.to_account_info(),
        to: ctx.accounts.deposit_token_account.to_account_info(),
        authority: ctx.accounts.escrow_account.to_account_info(),
    };
    let seeds = &[
        ESCROW_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[listing.escrow_bump],
    ];
    let signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, 1)?;

    Ok(())
}

pub fn repossess(ctx: Context<Repossess>) -> Result<()> {
    let listing = &mut ctx.accounts.loan_account;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp as u64;
    let loan_start_date = listing.start_date as u64;
    let loan_duration = unix_timestamp - loan_start_date;

    msg!("Loan start date: {} seconds", loan_start_date);
    msg!("Loan duration: {} seconds", listing.duration);
    msg!("Time passed: {} seconds", loan_duration);

    if listing.duration > loan_duration  {
        return Err(ErrorCode::NotOverdue.into())
    }
    
    listing.state = LoanState::Defaulted;

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.escrow_account.to_account_info(),
        to: ctx.accounts.lender_token_account.to_account_info(),
        authority: ctx.accounts.escrow_account.to_account_info(),
    };
    let seeds = &[
        ESCROW_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[listing.escrow_bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, 1)?;
    
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
    /// This is where we'll store the borrower's token
    #[account(
        init_if_needed,
        payer = borrower,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow_account,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Account<'info, Mint>,
    /// Misc
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
        constraint = loan_account.escrow == escrow_account.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Listed || loan_account.state == LoanState::Defaulted,
        close = borrower
    )]
    pub loan_account: Account<'info, Loan>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = loan_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
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
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = loan_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// Misc
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
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = loan_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
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
        constraint = loan_account.escrow == escrow_account.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
        close = borrower
    )]
    pub loan_account: Box<Account<'info, Loan>>,
    pub mint: Box<Account<'info, Mint>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Repossess<'info> {
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = loan_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(mut)]
    pub lender_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = loan_account.lender == lender.key(),
        constraint = loan_account.escrow == escrow_account.key(),
        constraint = loan_account.mint == mint.key(),
        constraint = loan_account.state == LoanState::Active,
    )]
    pub loan_account: Account<'info, Loan>,
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}
