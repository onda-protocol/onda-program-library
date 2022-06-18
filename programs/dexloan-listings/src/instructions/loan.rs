use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Listing, ListingState};
use crate::error::{ErrorCode};

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

pub const ESCROW_PREFIX: &'static [u8] = b"escrow";
pub const SECONDS_PER_YEAR: f64 = 31_536_000.0; 

pub fn init_loan(
    ctx: Context<InitLoan>,
    amount: u64,
    basis_points: u32,
    duration: u64
) -> Result<()> {
    let listing = &mut ctx.accounts.listing_account;
    // Init
    listing.mint = ctx.accounts.mint.key();
    listing.escrow = ctx.accounts.escrow_account.key();
    listing.bump = *ctx.bumps.get("listing_account").unwrap();
    listing.escrow_bump = *ctx.bumps.get("escrow_account").unwrap();
    //
    listing.amount = amount;
    listing.basis_points = basis_points;
    listing.duration = duration;
    listing.state = ListingState::Listed as u8;
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

pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
    let listing = &ctx.accounts.listing_account;
    let borrower = &ctx.accounts.borrower;
    let escrow_account = &ctx.accounts.escrow_account;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let token_program = &ctx.accounts.token_program;

    if escrow_account.amount == 0 {
        let cpi_program = token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Revoke {
            source: deposit_token_account.to_account_info(),
            authority: borrower.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::revoke(cpi_ctx)?;
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

pub fn make_loan(ctx: Context<MakeLoan>) -> Result<()> {
    let listing = &mut ctx.accounts.listing_account;

    listing.state = ListingState::Active as u8;
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

pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
    let listing = &mut ctx.accounts.listing_account;

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

pub fn repossess_collateral(ctx: Context<RepossessCollateral>) -> Result<()> {
    let listing = &mut ctx.accounts.listing_account;

    let unix_timestamp = ctx.accounts.clock.unix_timestamp as u64;
    let loan_start_date = listing.start_date as u64;
    let loan_duration = unix_timestamp - loan_start_date;

    msg!("Loan start date: {} seconds", loan_start_date);
    msg!("Loan duration: {} seconds", listing.duration);
    msg!("Time passed: {} seconds", loan_duration);

    if listing.duration > loan_duration  {
        return Err(ErrorCode::NotOverdue.into())
    }
    
    listing.state = ListingState::Defaulted as u8;

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

pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
    let listing = &mut ctx.accounts.listing_account;

    listing.close(ctx.accounts.borrower.to_account_info())?;

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
            Listing::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        space = Listing::space(),
    )]
    pub listing_account: Account<'info, Listing>,
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
pub struct CancelListing<'info> {
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            Listing::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = listing_account.bump,
        constraint = listing_account.borrower == *borrower.key,
        constraint = listing_account.escrow == escrow_account.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Listed as u8,
        close = borrower
    )]
    pub listing_account: Account<'info, Listing>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MakeLoan<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            Listing::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = listing_account.bump,
        constraint = listing_account.borrower == borrower.key(),
        constraint = listing_account.borrower != lender.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Listed as u8,
    )]
    pub listing_account: Account<'info, Listing>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = listing_account.escrow_bump,
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
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub lender: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            Listing::PREFIX,
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = listing_account.bump,
        constraint = listing_account.borrower == borrower.key(),
        constraint = listing_account.lender == lender.key(),
        constraint = listing_account.escrow == escrow_account.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Active as u8,
        close = borrower
    )]
    pub listing_account: Box<Account<'info, Listing>>,
    pub mint: Box<Account<'info, Mint>>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RepossessCollateral<'info> {
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(mut)]
    pub lender_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = listing_account.lender == lender.key(),
        constraint = listing_account.escrow == escrow_account.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Active as u8,
    )]
    pub listing_account: Account<'info, Listing>,
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,
    #[account(
        mut,
        constraint = listing_account.borrower == borrower.key(),
        constraint = listing_account.state != ListingState::Listed as u8,
        constraint = listing_account.state != ListingState::Active as u8,
    )]
    pub listing_account: Account<'info, Listing>,
}
