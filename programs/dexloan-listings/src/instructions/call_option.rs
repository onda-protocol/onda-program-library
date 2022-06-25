use anchor_lang::{
    prelude::*,
    solana_program::program_option::{COption}
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{CallOption, CallOptionState};
use crate::error::{ErrorCode};

const ESCROW_PREFIX: &'static [u8] = b"escrow";

pub fn init(
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
    call_option.state = CallOptionState::Listed;
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

pub fn buy(ctx: Context<BuyCallOption>) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option_account;

    call_option.state = CallOptionState::Active;
    call_option.buyer = ctx.accounts.buyer.key();
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

pub fn exercise(ctx: Context<ExerciseCallOption>) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    msg!("Strike price: {} lamports", call_option.strike_price);

    if unix_timestamp > call_option.expiry {
        return Err(ErrorCode::OptionExpired.into())
    }

    call_option.state = CallOptionState::Exercised;

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
        ESCROW_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[call_option.escrow_bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, 1)?;
    
    Ok(())
}

pub fn close(ctx: Context<CloseCallOption>) -> Result<()> {
    let call_option = &mut ctx.accounts.call_option_account;
    let escrow_account = &ctx.accounts.escrow_account;
    let deposit_token_account = &ctx.accounts.deposit_token_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;
    
    if call_option.state == CallOptionState::Active && call_option.expiry > unix_timestamp {
        return Err(ErrorCode::OptionNotExpired.into())
    }

    let cpi_program = ctx.accounts.token_program.to_account_info();

    if escrow_account.amount == 0 {
        if deposit_token_account.delegate == COption::Some(escrow_account.key()) {
            let cpi_accounts = anchor_spl::token::Revoke {
                source: deposit_token_account.to_account_info(),
                authority:  ctx.accounts.seller.to_account_info()
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            anchor_spl::token::revoke(cpi_ctx)?;
        }
    } else {
        let cpi_accounts = anchor_spl::token::Transfer {
            from: escrow_account.to_account_info(),
            to: deposit_token_account.to_account_info(),
            authority: escrow_account.to_account_info(),
        };
        let seeds = &[
            ESCROW_PREFIX,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[call_option.escrow_bump],
        ];
        let signer = &[&seeds[..]];
    
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        anchor_spl::token::transfer(cpi_ctx, 1)?;
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, strike_price: u64, expiry: i64)]
pub struct InitCallOption<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.owner == seller.key(),
        associated_token::mint = mint,
        associated_token::authority = seller,
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer = seller,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        space = CallOption::space(),
        bump,
    )]
    pub call_option_account: Account<'info, CallOption>,
    
    #[account(
        init_if_needed,
        payer = seller,
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
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
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
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.seller != buyer.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == CallOptionState::Listed,
    )]
    pub call_option_account: Account<'info, CallOption>,
    
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = deposit_token_account.amount == 1,
        constraint = deposit_token_account.delegate == COption::Some(escrow_account.key()),
        associated_token::mint = mint,
        associated_token::authority = seller,
    )]
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
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state != CallOptionState::Exercised,
        close = seller
    )]
    
    pub call_option_account: Account<'info, CallOption>,
    
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,
   
    #[account(
        mut,
        // constraint = deposit_token_account.delegate == COption::Some(escrow_account.key()),
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub deposit_token_account: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, Mint>,
    
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ExerciseCallOption<'info> {
    /// CHECK: contrained on listing_account
    #[account(mut)]
    pub seller: AccountInfo<'info>,
   
    #[account(mut)]
    pub buyer: Signer<'info>,
   
    #[account(
        mut,
        seeds = [
            CallOption::PREFIX,
            mint.key().as_ref(),
            seller.key().as_ref(),
        ],
        bump = call_option_account.bump,
        constraint = call_option_account.seller == seller.key(),
        constraint = call_option_account.buyer == buyer.key(),
        constraint = call_option_account.escrow == escrow_account.key(),
        constraint = call_option_account.mint == mint.key(),
        constraint = call_option_account.state == CallOptionState::Active,
    )]
    pub call_option_account: Account<'info, CallOption>,
    
    #[account(
        mut,
        seeds = [ESCROW_PREFIX, mint.key().as_ref()],
        bump = call_option_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, Mint>,
    
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}
