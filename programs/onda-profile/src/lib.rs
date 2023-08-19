use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Mint};
use mpl_token_metadata::{
    pda::find_metadata_account
};

declare_id!("ondapcq2qXTSynRieMCE9BjRsZ2XALEEZZunkwbhCPF");

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_PROFILE_SIZE: usize = 8 + 4 + MAX_NAME_LENGTH + 1 + 32;
pub const PROFILE_PREFIX: &str = "profile";

#[error_code]
pub enum OndaProfileError {
  #[msg("Unauthorized.")]
  Unauthorized,
}

#[account]
pub struct Profile {
    pub name: String,
    pub mint: Option<Pubkey>,
}

#[derive(Accounts)]
pub struct UpdateProfile<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(
        init_if_needed,
        seeds = [PROFILE_PREFIX.as_ref(), author.key().as_ref()],
        bump,
        payer = author,
        space = MAX_PROFILE_SIZE + 8,
    )]
    pub profile: Account<'info, Profile>,
    pub mint: Account<'info, Mint>,
    /// CHECK: deserialized
    pub metadata: UncheckedAccount<'info>,
    pub token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyProfile<'info> {
    /// CHECK: this is the user the profile belongs to
    pub author: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [PROFILE_PREFIX.as_ref(), author.key().as_ref()],
        bump,
        constraint = (
            profile.mint.is_some() && 
            profile.mint.unwrap() == mint.key() 
        ) @OndaProfileError::Unauthorized,
    )]
    pub profile: Account<'info, Profile>,
    pub mint: Account<'info, Mint>,
    /// CHECK: deserialized
    pub metadata: UncheckedAccount<'info>,
    pub token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod onda_profile {
    use super::*;

    pub fn update_profile(ctx: Context<UpdateProfile>, name: String) -> Result<()> {
        let author = &ctx.accounts.author;
        let profile = &mut ctx.accounts.profile;
        let mint = &ctx.accounts.mint;
        let metadata = &ctx.accounts.metadata;
        let token_account = &ctx.accounts.token_account;
        
        profile.name = puffed_out_string(&name, MAX_NAME_LENGTH);

        let is_valid_mint = user_owns_nft(
            &author.key(),
            &mint.key(),
            &metadata.key(),
            token_account
        );

        if is_valid_mint {
            profile.mint = Some(mint.key());
        }

        Ok(())
    }

    pub fn verify_profile(ctx: Context<VerifyProfile>) -> Result<()> {
        let author = &ctx.accounts.author;
        let profile = &mut ctx.accounts.profile;
        let mint = &ctx.accounts.mint;
        let metadata = &ctx.accounts.metadata;
        let token_account = &ctx.accounts.token_account;
        let is_valid_mint = user_owns_nft(
            &author.key(),
            &mint.key(),
            &metadata.key(),
            token_account
        );

        if is_valid_mint {
            msg!("verified profile for {:?}", author.key());
        } else {
            msg!("failed to verify profile for {:?}", author.key());
            profile.mint = None;
        }

        Ok(())
    }
}

/// Pads the string to the desired size with `0u8`s.
/// NOTE: it is assumed that the string's size is never larger than the given size.
pub fn puffed_out_string(s: &str, size: usize) -> String {
    let mut array_of_zeroes = vec![];
    let puff_amount = size - s.len();
    while array_of_zeroes.len() < puff_amount {
        array_of_zeroes.push(0u8);
    }
    s.to_owned() + std::str::from_utf8(&array_of_zeroes).unwrap()
}

pub fn user_owns_nft<'info>(
    user: &Pubkey,
    mint: &Pubkey,
    metadata: &Pubkey,
    token_account: &Account<'info, TokenAccount>,
) -> bool {
    let (metadata_pda, _) = find_metadata_account(
        &mint
      );
    
    if metadata_pda != metadata.key() {
        return false;
    }

    if token_account.mint != mint.key() {
        return false;
    }

    if token_account.amount != 1 {
        return false;
    } 

    if token_account.owner != user.key() {
        return false;
    }

    true
}
