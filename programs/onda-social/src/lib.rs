use anchor_lang::{
    prelude::*,
    solana_program::{keccak},
};
use spl_account_compression::{
    program::SplAccountCompression, wrap_application_data_v1, Node, Noop,
};

use crate::{
    error::OndaSocialError,
    state::{PostArgs, PostConfig, CommentArgs, LeafSchema, RestrictionType, ASSET_PREFIX, POST_CONFIG_SIZE},
};

pub mod error;
pub mod state;

declare_id!("62616yhPNbv1uxcGbs84pk9PmGbBaaEBXAZmLE6P1nGS");

#[program]
pub mod onda_social {
    use super::*;

    pub fn create_post(
        ctx: Context<CreatePost>,
        post: PostArgs,
    ) -> Result<()> {
        let author =  ctx.accounts.author.key();
        let post_config = &mut ctx.accounts.post_config;
        let post_config_bump = *ctx.bumps.get("post_config").unwrap();
        let merkle_tree = &ctx.accounts.merkle_tree;
        let seed = merkle_tree.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("post_config").unwrap()]];
        let wrapper = &ctx.accounts.log_wrapper;
        let compression_program = &ctx.accounts.compression_program;
        post_config.set_inner(PostConfig {
            author,
            total_capacity: 1 << post.max_depth,
            post_count: 0,
            restriction: match post.collection {
                Some(collection) => RestrictionType::Collection { collection },
                None => RestrictionType::None,
            },
        });
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::Initialize {
                authority: post_config.to_account_info(),
                merkle_tree: merkle_tree.to_account_info(),
                noop: wrapper.to_account_info(),
            },
            authority_pda_signer,
        );
        spl_account_compression::cpi::init_empty_merkle_tree(
            cpi_ctx,
            post.max_depth,
            post.max_buffer_size
        )?;

        let data_hash = keccak::hashv(&[post.post_data.try_to_vec()?.as_slice()]);
        append_post(
            author,
            data_hash,
            post_config,
            post_config_bump,
            merkle_tree,
            compression_program,
            wrapper,
        )?;

        Ok(())
    }

    pub fn add_comment(ctx: Context<AddComment>, data: CommentArgs) -> Result<()> {
        let author = ctx.accounts.author.key();
        let post_config = &mut ctx.accounts.post_config;
        let post_config_bump = *ctx.bumps.get("post_config").unwrap();
        let merkle_tree = &ctx.accounts.merkle_tree;
        let wrapper = &ctx.accounts.log_wrapper;
        let compression_program = &ctx.accounts.compression_program;

        if !post_config.contains_post_capacity(1) {
            return err!(OndaSocialError::InsufficientPostCapacity);
        }

        let data_hash = keccak::hashv(&[data.try_to_vec()?.as_slice()]);
        append_post(
            author,
            data_hash,
            post_config,
            post_config_bump,
            merkle_tree,
            compression_program,
            wrapper,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(data: PostArgs)]
pub struct CreatePost<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = author,
        space = POST_CONFIG_SIZE,
        bump,
    )]
    pub post_config: Account<'info, PostConfig>,
    #[account(zero)]
    /// CHECK: This account must be all zeros
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data: CommentArgs)]
pub struct AddComment<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub post_config: Account<'info, PostConfig>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

pub fn append_post<'info>(
    author: Pubkey,
    data_hash: keccak::Hash,
    post_config: &mut Account<'info, PostConfig>,
    post_config_bump: u8,
    merkle_tree: &AccountInfo<'info>,
    compression_program: &AccountInfo<'info>,
    log_wrapper: &Program<'info, Noop>,
) -> Result<()> {
    let asset_id = get_asset_id(&merkle_tree.key(), post_config.post_count);
    let created_at = Clock::get()?.unix_timestamp;
    let leaf = LeafSchema::new_v0(
        asset_id,
        author,
        created_at,
        None,
        post_config.post_count,
        data_hash.to_bytes(),
    );

    wrap_application_data_v1(leaf.to_event().try_to_vec()?, log_wrapper)?;

    append_leaf(
        &merkle_tree.key(),
        post_config_bump,
        &compression_program.to_account_info(),
        &post_config.to_account_info(),
        &merkle_tree.to_account_info(),
        &log_wrapper.to_account_info(),
        leaf.to_node(),
    )?;

    post_config.increment_post_count();

    Ok(())
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

pub fn get_asset_id(tree_id: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            ASSET_PREFIX.as_ref(),
            tree_id.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::id(),
    )
    .0
}