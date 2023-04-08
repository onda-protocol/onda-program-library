use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak
    },
};
use spl_account_compression::{
    program::SplAccountCompression, wrap_application_data_v1, Node, Noop,
};

use crate::{
    error::OndaSocialError,
    state::{PostArgs, TreeConfig, LeafSchema, ASSET_PREFIX, TREE_AUTHORITY_SIZE},
};

pub mod error;
pub mod state;

declare_id!("62616yhPNbv1uxcGbs84pk9PmGbBaaEBXAZmLE6P1nGS");

#[program]
pub mod onda_social {
    use super::*;

    pub fn create_tree(
        ctx: Context<CreateTree>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let merkle_tree = ctx.accounts.merkle_tree.to_account_info();
        let seed = merkle_tree.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("tree_authority").unwrap()]];
        let authority = &mut ctx.accounts.tree_authority;
        authority.set_inner(TreeConfig {
            tree_creator: ctx.accounts.tree_creator.key(),
            tree_delegate: ctx.accounts.tree_creator.key(),
            total_post_capacity: 1 << max_depth,
            post_count: 0,
        });
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::Initialize {
                authority: ctx.accounts.tree_authority.to_account_info(),
                merkle_tree,
                noop: ctx.accounts.log_wrapper.to_account_info(),
            },
            authority_pda_signer,
        );
        spl_account_compression::cpi::init_empty_merkle_tree(cpi_ctx, max_depth, max_buffer_size)
    }

    pub fn add_post(ctx: Context<AddPost>, post: PostArgs) -> Result<()> {
        let owner = ctx.accounts.leaf_owner.key();
        let delegate = ctx.accounts.leaf_delegate.key();
        let authority = &mut ctx.accounts.tree_authority;
        let authority_bump = *ctx.bumps.get("tree_authority").unwrap();
        let merkle_tree = &ctx.accounts.merkle_tree;
        let wrapper = &ctx.accounts.log_wrapper;
        let compression_program = &ctx.accounts.compression_program;

        if !authority.contains_post_capacity(1) {
            return err!(OndaSocialError::InsufficientPostCapacity);
        }

        let data_hash = keccak::hashv(&[post.try_to_vec()?.as_slice()]);
        let asset_id = get_asset_id(&merkle_tree.key(), authority.post_count);
        let leaf = LeafSchema::new_v0(
            asset_id,
            owner,
            delegate,
            authority.post_count,
            data_hash.to_bytes(),
        );

        wrap_application_data_v1(leaf.to_event().try_to_vec()?, wrapper)?;

        append_leaf(
            &merkle_tree.key(),
            authority_bump,
            &compression_program.to_account_info(),
            &authority.to_account_info(),
            &merkle_tree.to_account_info(),
            &wrapper.to_account_info(),
            leaf.to_node(),
        )?;

        authority.increment_post_count();

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTree<'info> {
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = TREE_AUTHORITY_SIZE,
        bump,
    )]
    pub tree_authority: Account<'info, TreeConfig>,
    #[account(zero)]
    /// CHECK: This account must be all zeros
    pub merkle_tree: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub tree_creator: Signer<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPost<'info> {
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub tree_authority: Account<'info, TreeConfig>,
    /// CHECK: This account is neither written to nor read from.
    pub leaf_owner: AccountInfo<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub leaf_delegate: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_tree: UncheckedAccount<'info>,
    pub payer: Signer<'info>,
    pub tree_delegate: Signer<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
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