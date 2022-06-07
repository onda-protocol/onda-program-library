use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

const LISTING_PREFIX: &str = "listing";
const ESCROW_PREFIX: &str = "escrow";

#[program]
pub mod dexloan_listings {
    use super::*;

    pub const SECONDS_PER_YEAR: f64 = 31_536_000.0; 

    pub fn init_listing(
        ctx: Context<InitListing>,
        options: ListingOptions
    ) -> Result<()> {
        let listing = &mut ctx.accounts.listing_account;
        // Init
        listing.mint = ctx.accounts.mint.key();
        listing.escrow = ctx.accounts.escrow_account.key();
        listing.authority = ctx.accounts.authority.key();
        listing.bump = *ctx.bumps.get("listing_account").unwrap();
        listing.escrow_bump = *ctx.bumps.get("escrow_account").unwrap();
        // List
        listing.amount = options.amount;
        listing.listing_type = options.listing_type;
        listing.state = ListingState::Listed as u8;

        match listing.listing_type {
            // Loan
            0 => {
                listing.basis_points = options.basis_points;
                listing.duration = options.duration;
            },
            // Call Option
            1 => {
                listing.end_date = options.end_date;
                listing.strike_price = options.strike_price;
            },
            _ => {
                return Err(ErrorCode::InvalidListingType.into());
            }                
        }

        // Transfer
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Approve {
            to: ctx.accounts.deposit_token_account.to_account_info(),
            delegate: ctx.accounts.escrow_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::approve(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Revoke {
            source: ctx.accounts.deposit_token_account.to_account_info(),
            authority: ctx.accounts.borrower.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::revoke(cpi_ctx)?;
        
        Ok(())
    }

    pub fn take_listing(ctx: Context<TakeListing>) -> Result<()> {
        let listing = &mut ctx.accounts.listing_account;

        listing.state = ListingState::Active as u8;
        listing.buyer = ctx.accounts.buyer.key();
        listing.start_date = ctx.accounts.clock.unix_timestamp;


        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.escrow_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[listing.escrow_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
        
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &listing.authority,
                &listing.buyer,
                listing.amount,
            ),
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.buyer.to_account_info(),
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
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[listing.escrow_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn exercise_option(ctx: Context<ExerciseOption>) -> Result<()> {
        let listing = &mut ctx.accounts.listing_account;

        match listing.listing_type {
            // Loan
            0 => {
                let unix_timestamp = ctx.accounts.clock.unix_timestamp as u64;
                let loan_start_date = listing.start_date as u64;
                let loan_duration = unix_timestamp - loan_start_date;

                if listing.duration > loan_duration  {
                    return Err(ErrorCode::NotOverdue.into())
                }
            },
            // Call Option
            1 => {
                let unix_timestamp = ctx.accounts.clock.unix_timestamp;

                if unix_timestamp > listing.end_date {
                    return Err(ErrorCode::OptionExpired.into())
                }

                anchor_lang::solana_program::program::invoke(
                    &anchor_lang::solana_program::system_instruction::transfer(
                        &listing.buyer,
                        &listing.authority,
                        listing.amount,
                    ),
                    &[
                        ctx.accounts.authority.to_account_info(),
                        ctx.accounts.buyer.to_account_info(),
                    ]
                )?;
            },
            _ => {
                return Err(ErrorCode::InvalidListingType.into());
            }
        }
        
        listing.state = ListingState::Exercised as u8;

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.escrow_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX.as_bytes(),
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
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Clone)]
pub struct ListingOptions {
    amount: u64,
    duration: u64,
    end_date: i64,
    basis_points: u32,
    strike_price: u64,
    listing_type: u8,
}

#[derive(Accounts)]
#[instruction(options: ListingOptions)]
pub struct InitListing<'info> {
    /// The person who is listing the loan
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.mint == mint.key(),
        constraint = deposit_token_account.owner == authority.key(),
        constraint = deposit_token_account.amount == 1
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    /// The new listing account
    #[account(
        init,
        payer = authority,
        seeds = [
            LISTING_PREFIX.as_bytes(),
            mint.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump,
        space = LISTING_SIZE,
    )]
    pub listing_account: Account<'info, ListingV2>,
    /// This is where we'll store the borrower's token
    #[account(
        init_if_needed,
        payer = authority,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
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
    #[account(mut)]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            LISTING_PREFIX.as_bytes(),
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
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TakeListing<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub authority: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            LISTING_PREFIX.as_bytes(),
            mint.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump = listing_account.bump,
        constraint = listing_account.authority == authority.key(),
        constraint = listing_account.authority != buyer.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Listed as u8,
    )]
    pub listing_account: Account<'info, ListingV2>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(mut)]
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
    #[account(mut)]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub lender: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            LISTING_PREFIX.as_bytes(),
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
pub struct ExerciseOption<'info> {
    #[account(mut)]
    authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            LISTING_PREFIX.as_bytes(),
            mint.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump = listing_account.bump,
        constraint = listing_account.buyer == buyer.key(),
        constraint = listing_account.escrow == escrow_account.key(),
        constraint = listing_account.mint == mint.key(),
        constraint = listing_account.state == ListingState::Active as u8,
    )]
    pub listing_account: Account<'info, ListingV2>,
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

const LISTING_SIZE: usize = 8 + // key
1 + // state
8 + // amount
32 + // borrower
32 + // lender
4 + // basis_points
8 + // duration
8 + // start_date
32 + // escrow
32 + // mint
1 + // bump
1 + // escrow bump
120; // padding

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub enum ListingState {
    Listed = 1,
    Active = 2,
    Exercised = 5,
}

pub enum ListingType {
    Loan = 0,
    CallOption = 1,
}

#[account]
pub struct Listing {
    /// Whether the loan is active
    pub state: u8,
    /// The amount of the loan
    pub amount: u64,
    /// The NFT holder
    pub borrower: Pubkey,
    /// The issuer of the loan
    pub lender: Pubkey,
    /// Annualized return
    pub basis_points: u32,
    /// Duration of the loan in seconds
    pub duration: u64,
    /// The start date of the loan
    pub start_date: i64,
    /// The escrow where the collateral NFT is held
    pub escrow: Pubkey,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// Misc
    pub bump: u8,
    pub escrow_bump: u8,
    // New fields
    // The type of the listing
    // 0: loan, 1: call
    pub listing_type: u8,
    // The exercise price of the call option
    pub exercise_price: u64,
}

const LISTING_V2_SIZE: usize = 8 + // key
1 + // state
8 + // amount
8 + // strike_price
32 + // authority
32 + // buyer
4 + // basis_points
8 + // duration
8 + // start_date
8 + // end_date
32 + // escrow
32 + // mint
1 + // bump
1 + // escrow bump
90; // padding

#[account]
pub struct ListingV2 {
    /// Whether the loan is active
    pub state: u8,
    /// The amount of the loan
    pub amount: u64,
    /// The exercise price of the call option
    pub strike_price: u64,
    /// The NFT holder
    pub authority: Pubkey,
    /// The issuer of the loan or the buyer of the call option
    pub buyer: Pubkey,
    /// Annualized return
    pub basis_points: u32,
    /// Duration of the loan in seconds
    pub duration: u64,
    /// The start date of the loan
    pub start_date: i64,
    /// The end date of the option
    pub end_date: i64,
    /// The escrow where the collateral NFT is held
    pub escrow: Pubkey,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    // The type of the listing
    // 0: loan, 1: call
    pub listing_type: u8,
    /// Misc
    pub bump: u8,
    pub escrow_bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("This loan is not overdue")]
    NotOverdue,
    #[msg("Invalid state")]
    InvalidState,
    #[msg("Invalid listing type")]
    InvalidListingType,
    #[msg("Option expired")]
    OptionExpired,
}