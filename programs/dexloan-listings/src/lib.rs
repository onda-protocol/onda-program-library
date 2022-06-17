use anchor_lang::prelude::*;
use anchor_lang::AccountsClose;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj");

const LISTING_PREFIX: &str = "listing";
const CALL_OPTION_PREFIX: &str = "call_option";
const ESCROW_PREFIX: &str = "escrow";

#[program]
pub mod dexloan_listings {
    use super::*;

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

    pub fn init_call_option(
        ctx: Context<InitCallOption>,
        amount: u64,
        strike_price: u64,
        expiry: i64
    ) -> Result<()> {
        let call_option = &mut ctx.accounts.call_option_account;
        let unix_timestamp = ctx.accounts.clock.unix_timestamp;
        msg!("unix_timestamp: {} seconds", unix_timestamp);
        msg!("expiry: {} seconds", expiry);
        if unix_timestamp > expiry {
            return Err(ErrorCode::InvalidExpiry.into())
        }

        // Init
        call_option.escrow = ctx.accounts.escrow_account.key();
        call_option.seller = ctx.accounts.seller.key();
        call_option.mint = ctx.accounts.mint.key();
        call_option.bump = *ctx.bumps.get("call_option_account").unwrap();
        call_option.escrow_bump = *ctx.bumps.get("escrow_account").unwrap();
        //
        call_option.amount = amount;
        call_option.expiry = expiry;
        call_option.strike_price = strike_price;
        call_option.state = ListingState::Listed as u8;
        // Delegate authority
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Approve {
            to: ctx.accounts.deposit_token_account.to_account_info(),
            delegate: ctx.accounts.escrow_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
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
                ESCROW_PREFIX.as_bytes(),
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
            ESCROW_PREFIX.as_bytes(),
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

    pub fn buy_call_option(ctx: Context<BuyCallOption>) -> Result<()> {
        let call_option = &mut ctx.accounts.call_option_account;

        call_option.state = ListingState::Active as u8;
        call_option.buyer = ctx.accounts.buyer.key();
        // Transfer token to escrow
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.escrow_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[call_option.escrow_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
        // Transfer option cost
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &call_option.buyer,
                &call_option.seller,
                call_option.amount,
            ),
            &[
                ctx.accounts.seller.to_account_info(),
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
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[listing.escrow_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
        
        Ok(())
    }

    pub fn exercise_call_option(ctx: Context<ExerciseOption>) -> Result<()> {
        let call_option = &mut ctx.accounts.call_option_account;
        let unix_timestamp = ctx.accounts.clock.unix_timestamp;

        msg!("Strike price: {} lamports", call_option.strike_price);

        if unix_timestamp > call_option.expiry {
            return Err(ErrorCode::OptionExpired.into())
        }

        call_option.state = ListingState::Exercised as u8;

        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &call_option.buyer,
                &call_option.seller,
                call_option.strike_price,
            ),
            &[
                ctx.accounts.seller.to_account_info(),
                ctx.accounts.buyer.to_account_info(),
            ]
        )?;

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.escrow_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[call_option.escrow_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
        
        Ok(())
    }

    pub fn close_call_option(ctx: Context<CloseCallOption>) -> Result<()> {
        let call_option = &mut ctx.accounts.call_option_account;
        let unix_timestamp = ctx.accounts.clock.unix_timestamp;

        if call_option.expiry > unix_timestamp {
            return Err(ErrorCode::OptionNotExpired.into())
        }

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.escrow_account.to_account_info(),
            to: ctx.accounts.deposit_token_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX.as_bytes(),
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[call_option.escrow_bump],
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
            LISTING_PREFIX.as_bytes(),
            mint.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump,
        space = LISTING_SIZE,
    )]
    pub listing_account: Account<'info, Listing>,
    /// This is where we'll store the borrower's token
    #[account(
        init_if_needed,
        payer = borrower,
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
#[instruction(amount: u64, strike_price: u64, expiry: i64)]
pub struct InitCallOption<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
        mut,
        constraint = deposit_token_account.mint == mint.key(),
        constraint = deposit_token_account.owner == seller.key(),
        constraint = deposit_token_account.amount == 1
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = seller,
        seeds = [
            CALL_OPTION_PREFIX.as_bytes(),
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump,
        space = CALL_OPTION_SIZE,
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        init_if_needed,
        payer = seller,
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
    pub clock: Sysvar<'info, Clock>,
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
            LISTING_PREFIX.as_bytes(),
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
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = listing_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == borrower.key(),
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct BuyCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            CALL_OPTION_PREFIX.as_bytes(),
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.seller != buyer.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == ListingState::Listed as u8,
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
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
pub struct CloseCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: Signer<'info>,
    /// The listing the loan is being issued against
    #[account(
        mut,
        seeds = [
            CALL_OPTION_PREFIX.as_bytes(),
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == ListingState::Listed as u8,
        close = seller
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = deposit_token_account.owner == seller.key(),
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
pub struct RepossessCollateral<'info> {
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
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
pub struct ExerciseOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            CALL_OPTION_PREFIX.as_bytes(),
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.buyer == buyer.key(),
        constraint = call_option_account.escrow == escrow_account.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == ListingState::Active as u8,
    )]
    pub call_option_account: Account<'info, CallOption>,
    #[account(
        mut,
        seeds = [ESCROW_PREFIX.as_bytes(), mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
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

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub enum ListingState {
    Listed = 1,
    Active = 2,
    Exercised = 4,
    Defaulted = 5,
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
90; // padding

#[account]
pub struct CallOption {
    /// Whether the option is active
    pub state: u8,
    /// The amount of the loan
    pub amount: u64,
    /// The issuer of the call option
    pub seller: Pubkey,
    /// The buyer of the call option
    pub buyer: Pubkey,
    /// Duration of the loan in seconds
    pub expiry: i64,
    /// The start date of the loan
    pub strike_price: u64,
    /// The escrow where the collateral NFT is held
    pub escrow: Pubkey,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// Misc
    pub bump: u8,
    pub escrow_bump: u8,
}

const CALL_OPTION_SIZE: usize = 8 + // key
1 + // state
8 + // amount
32 + // seller
32 + // buyer
8 + // expiry
8 + // exercise price
32 + // escrow
32 + // mint
1 + // bump
1 + // escrow bump
90; // padding

#[error_code]
pub enum ErrorCode {
    #[msg("This loan is not overdue")]
    NotOverdue,
    #[msg("Invalid expiry")]
    InvalidExpiry,
    #[msg("Invalid state")]
    InvalidState,
    #[msg("Invalid listing type")]
    InvalidListingType,
    #[msg("Option expired")]
    OptionExpired,
    #[msg("Option not expired")]
    OptionNotExpired,
}