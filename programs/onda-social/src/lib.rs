use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak, 
        system_instruction,
    },
};
use anchor_spl::{
    token::{Mint, TokenAccount},
};
use spl_account_compression::{
    program::SplAccountCompression, wrap_application_data_v1, Node, Noop,
};
use mpl_token_metadata::{
    state::Metadata,
    pda::find_metadata_account
};

use crate::{
    error::OndaSocialError,
    state::*,
};

pub mod error;
pub mod state;

declare_id!("9JraSM3unmzqJ44RD8bmxmL4iu9tJfR7U7tv6EkcP63s");

#[derive(Accounts)]
pub struct InitForum<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = FORUM_CONFIG_SIZE,
        bump,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    #[account(zero)]
    /// CHECK: This account must be all zeros
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddEntry<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    pub mint: Option<Account<'info, Mint>>,
    /// CHECK: deserialized
    pub metadata: Option<UncheckedAccount<'info>>,
    pub token_account: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: constrained by seeds
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(entry_id: Pubkey)]
pub struct LikeEntry<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    /// CHECK: constrained by seeds
    pub author: AccountInfo<'info>,
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    #[account(
        init_if_needed,
        seeds = [LIKES_PREFIX.as_ref(), entry_id.as_ref(), author.key().as_ref()],
        bump,
        payer = payer,
        space = LIKES_SIZE,
    )]
    pub like_record: Account<'info, LikeRecord>,
    #[account(mut)]
    /// CHECK: constrained by seeds
    pub merkle_tree: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
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
        ) @OndaSocialError::Unauthorized,
    )]
    pub profile: Account<'info, Profile>,
    pub mint: Account<'info, Mint>,
    /// CHECK: deserialized
    pub metadata: UncheckedAccount<'info>,
    pub token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod onda_social {
    use anchor_lang::solana_program::program::invoke;

    use crate::state::DataV1;

    use super::*;

    pub fn init_forum(
        ctx: Context<InitForum>,
        max_depth: u32,
        max_buffer_size: u32,
        restriction: RestrictionType,
    ) -> Result<()> {
        let forum_config = &mut ctx.accounts.forum_config;
        let merkle_tree = &ctx.accounts.merkle_tree;
        let seed = merkle_tree.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("forum_config").unwrap()]];
        let wrapper = &ctx.accounts.log_wrapper;
        forum_config.set_inner(ForumConfig {
            total_capacity: 1 << max_depth,
            post_count: 0,
            restriction,
        });
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::Initialize {
                authority: forum_config.to_account_info(),
                merkle_tree: merkle_tree.to_account_info(),
                noop: wrapper.to_account_info(),
            },
            authority_pda_signer,
        );
        spl_account_compression::cpi::init_empty_merkle_tree(
            cpi_ctx,
            max_depth,
            max_buffer_size
        )
    }

    pub fn add_entry(
        ctx: Context<AddEntry>,
        data: DataV1,
    ) -> Result<()> {
        let author =  ctx.accounts.author.key();
        let forum_config = &mut ctx.accounts.forum_config;
        let forum_config_bump = *ctx.bumps.get("forum_config").unwrap();
        let merkle_tree = &ctx.accounts.merkle_tree;
        let log_wrapper = &ctx.accounts.log_wrapper;
        let compression_program = &ctx.accounts.compression_program;

        let mint = &ctx.accounts.mint;
        let metadata = &ctx.accounts.metadata;
        let token_account = &ctx.accounts.token_account;
        // Check if the forum is restricted to a collection.
        match forum_config.restriction {
            RestrictionType::None => (),
            RestrictionType::Mint { address } => {
                // TODO: Check the mint is the same as the one in the forum config
            },
            RestrictionType::Collection { address } => {
                let mint = mint.clone().ok_or(OndaSocialError::Unauthorized)?;
                let metadata_info = metadata.clone().ok_or(OndaSocialError::Unauthorized)?;
                let metadata = Metadata::deserialize(
                    &mut metadata_info.data.borrow_mut().as_ref()
                )?;
                let token_account = token_account.clone().ok_or(OndaSocialError::Unauthorized)?;

                // Check the metadata address is valid pda for this mint
                let (metadata_pda, _) = find_metadata_account(
                    &mint.key()
                  );
                require_keys_eq!(metadata_info.key(), metadata_pda, OndaSocialError::Unauthorized);

                // Check the metadata is verified for this collection
                let metadata_collection = metadata.collection.ok_or(OndaSocialError::Unauthorized)?;
                require!(metadata_collection.verified, OndaSocialError::Unauthorized);
                require_keys_eq!(address, metadata_collection.key, OndaSocialError::Unauthorized);

                // Check if the token account is owned by the author.
                require_keys_eq!(author, token_account.owner, OndaSocialError::Unauthorized);
            }
        }
        let entry_id = get_entry_id(&merkle_tree.key(), forum_config.post_count);
        let created_at = Clock::get()?.unix_timestamp;
        let data_hash = keccak::hashv(&[&data.try_to_vec()?]);
        let leaf = LeafSchema::new_v0(
            entry_id,
            author,
            created_at,
            None,
            forum_config.post_count,
            data_hash.to_bytes(),
        );

        wrap_application_data_v1(leaf.to_event().try_to_vec()?, log_wrapper)?;

        append_leaf(
            &merkle_tree.key(),
            forum_config_bump,
            &compression_program.to_account_info(),
            &forum_config.to_account_info(),
            &merkle_tree.to_account_info(),
            &log_wrapper.to_account_info(),
            leaf.to_node(),
        )?;

        forum_config.increment_post_count();

        Ok(())
    }

    pub fn like_entry(ctx: Context<LikeEntry>, entry_id: Pubkey) -> Result<()> {
        let payer = &ctx.accounts.payer;
        let author = &ctx.accounts.author;
        let like_record = &mut ctx.accounts.like_record;

        like_record.increment_like_count();

        msg!("tipping like to author {:?} for entry ${:?}", author.key(), entry_id.key());
        // 1 like == 100,000 lamports
        invoke(
            &system_instruction::transfer(
                &payer.key(),
                &author.key(),
                LIKE_AMOUNT_LAMPORTS,
            ),
            &[
                payer.to_account_info(),
                author.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

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

pub fn append_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    compression_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_tree: &AccountInfo<'info>,
    log_wrapper: &AccountInfo<'info>,
    leaf_node: Node,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        compression_program.clone(),
        spl_account_compression::cpi::accounts::Modify {
            authority: authority.clone(),
            merkle_tree: merkle_tree.clone(),
            noop: log_wrapper.clone(),
        },
        authority_pda_signer,
    );
    spl_account_compression::cpi::append(cpi_ctx, leaf_node)
}

pub fn get_entry_id(tree_id: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            ENTRY_PREFIX.as_ref(),
            tree_id.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::id(),
    )
    .0
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