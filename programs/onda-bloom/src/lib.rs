use anchor_lang::{prelude::*};
use solana_program::{pubkey, pubkey::Pubkey};
use anchor_spl::token::{self, TokenAccount, Transfer, Token, Mint};

declare_id!("onda3Sxku2NT88Ho8WfEgbkavNEELWzaguvh4itdn3C");

pub const BLOOM_PREFIX: &str = "bloom";
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

#[derive(Accounts)]
#[instruction(entry_id: Pubkey, amount: u64)]
pub struct FeedPlankton<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = payer,
    )]
    deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = payer.key() != author.key() @OndaBloomError::Unauthorized,
    )]
    /// CHECK: constrained by seeds
    pub author: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        token::mint = mint,
        token::authority = author,
        payer = payer,
    )]
    pub author_token_account: Box<Account<'info, TokenAccount>>,
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
}

#[program]
pub mod onda_bloom {
    use super::*;

    pub fn feed_plankton(ctx: Context<FeedPlankton>, _entry_id: Pubkey, amount: u64) -> Result<()> {
        let payer = &ctx.accounts.payer;
        let bloom = &mut ctx.accounts.bloom;

        bloom.increment_plankton_count();

        // Protocol takes 2% of the amount 
        let protocol_fee = amount.checked_div(50).ok_or(OndaBloomError::NumericOverflow).unwrap();
        let remaining_amount = amount.checked_sub(protocol_fee).ok_or(OndaBloomError::NumericOverflow).unwrap();

        let cpi_accounts = Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.protocol_fee_token_account.to_account_info(),
            authority: payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, protocol_fee)?;

        let cpi_accounts = Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.author_token_account.to_account_info(),
            authority: payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, remaining_amount)?;

        Ok(())
    }
}
