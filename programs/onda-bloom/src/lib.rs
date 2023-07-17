use anchor_lang::prelude::*;
use solana_program::{pubkey, pubkey::Pubkey};
use anchor_spl::{
    token::{self, TokenAccount, Transfer, Token, Mint},
    associated_token::AssociatedToken
};
use gpl_session::{SessionError, SessionToken, session_auth_or, Session};

declare_id!("onda3Sxku2NT88Ho8WfEgbkavNEELWzaguvh4itdn3C");

pub const BLOOM_PREFIX: &str = "bloom";
pub const ESCROW_PREFIX: &str = "escrow";
pub const REWARD_PREFIX: &str = "reward_escrow";
pub const CLAIM_MARKER_PREFIX: &str = "claim_marker";
pub const BLOOM_SIZE: usize = 8 + 8;
pub const PLANKTON_MINT: Pubkey = pubkey!("pktnre2sUNQZXwHicZj6njpShhSazmzQz5rJtcqnkG5");
pub const PROTOCOL_FEE_PLANKTON_ATA: Pubkey = pubkey!("EneovF7KrWHBC6QKmoiwC2S6PUFBZnpYcuyUTdD59iYp");

#[error_code]
pub enum OndaBloomError {
    #[msg("Unauthorized.")]
    Unauthorized,
    #[msg("Invalid mint.")]
    InvalidMint,
    #[msg("Numeric overflow.")]
    NumericOverflow,
} 

#[account]
pub struct Bloom {
    pub plankton: u64,
}

impl Bloom {
    pub fn increment_plankton_count(&mut self) {
        self.plankton = self.plankton.saturating_add(1);
    }
}


#[account]
pub struct ClaimMarker {
    pub marker: u8,
}

impl ClaimMarker {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();
}

#[derive(Accounts, Session)]
#[instruction(entry_id: Pubkey, amount: u64)]
pub struct FeedPlankton<'info> {
    /// CHECK: session token
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    #[session(
        // The ephemeral keypair signing the transaction
        signer = signer,
        // The authority of the user account which must have created the session
        authority = payer.key()
    )]
    // Session Tokens are passed as optional accounts
    pub session_token: Option<Account<'info, SessionToken>>,
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds=[ESCROW_PREFIX.as_ref(), mint.key().as_ref(), payer.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = deposit_token_account,
    )]
    deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        constraint = payer.key() != author.key() @OndaBloomError::Unauthorized,
    )]
    /// CHECK: constrained by seeds
    pub author: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        seeds=[ESCROW_PREFIX.as_ref(),  mint.key().as_ref(), author.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow_token_account,
        payer = payer,
    )]
    pub escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        seeds = [BLOOM_PREFIX.as_ref(), entry_id.as_ref(), author.key().as_ref()],
        bump,
        payer = payer,
        space = BLOOM_SIZE,
    )]
    pub bloom: Account<'info, Bloom>,
    #[account(
        mut,
        token::mint = mint,
        constraint = protocol_fee_token_account.key() == PROTOCOL_FEE_PLANKTON_ATA,
    )]
    pub protocol_fee_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        constraint = mint.key() == PLANKTON_MINT @OndaBloomError::InvalidMint,
    )]
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct ClaimPlankton<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        seeds=[ESCROW_PREFIX.as_ref(), mint.key().as_ref(), signer.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow_token_account,
        payer = signer,
    )]
    pub escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds=[REWARD_PREFIX.as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = reward_token_account,
    )]
    pub reward_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        seeds=[BLOOM_PREFIX.as_ref(), signer.key().as_ref()],
        bump,
        payer = signer,
        space = ClaimMarker::SIZE
    )]
    pub claim_marker: Box<Account<'info, ClaimMarker>>, 
    #[account(
        constraint = mint.key() == PLANKTON_MINT @OndaBloomError::InvalidMint,
    )]
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        constraint = mint.key() == PLANKTON_MINT @OndaBloomError::InvalidMint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds=[REWARD_PREFIX.as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = reward_token_account,
        payer = signer,
    )]
    pub reward_token_account: Box<Account<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[program]
pub mod onda_bloom {
    use super::*;

    #[session_auth_or(
        ctx.accounts.author.key() == ctx.accounts.signer.key(),
        OndaBloomError::Unauthorized
    )]
    pub fn feed_plankton(ctx: Context<FeedPlankton>, _entry_id: Pubkey, amount: u64) -> Result<()> {
        let bloom = &mut ctx.accounts.bloom;

        bloom.increment_plankton_count();

        // Protocol takes 2% of the amount 
        let protocol_fee = amount.checked_div(50).ok_or(OndaBloomError::NumericOverflow).unwrap();
        let remaining_amount = amount.checked_sub(protocol_fee).ok_or(OndaBloomError::NumericOverflow).unwrap();

        let payer_key = ctx.accounts.payer.key();
        let mint_key = ctx.accounts.mint.key();
        let seeds = &[
            ESCROW_PREFIX.as_ref(),
            mint_key.as_ref(),
            payer_key.as_ref(),
            &[*ctx.bumps.get("deposit_token_account").unwrap()]
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.protocol_fee_token_account.to_account_info(),
            authority: ctx.accounts.deposit_token_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, protocol_fee)?;

        let cpi_accounts = Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.deposit_token_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, remaining_amount)?;

        Ok(())
    }

    pub fn claim_plankton(ctx: Context<ClaimPlankton>) -> Result<()> {
        let mint_key = ctx.accounts.mint.key();
        let seeds = &[
            REWARD_PREFIX.as_ref(),
            mint_key.as_ref(),
            &[*ctx.bumps.get("reward_token_account").unwrap()]
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.reward_token_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, 1000)?;

        Ok(())
    }

    pub fn init(ctx: Context<Init>) -> Result<()> {
        Ok(())
    }
}
