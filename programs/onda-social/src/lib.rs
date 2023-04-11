use anchor_lang::{
    prelude::*,
    solana_program::{keccak},
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
    state::{DataV1, ForumConfig, LeafSchema, RestrictionType, ENTRY_PREFIX, FORUM_CONFIG_SIZE},
};

pub mod error;
pub mod state;

declare_id!("BWWPkJpv6fV2ZM5aNua8btxBXooWdW2qjWwUDBhz1p9S");

#[program]
pub mod onda_social {
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
            RestrictionType::Collection { collection } => {
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
                require_keys_eq!(collection, metadata_collection.key, OndaSocialError::Unauthorized);

                // Check if the token account is owned by the author.
                require_keys_eq!(author, token_account.owner, OndaSocialError::Unauthorized);

                return err!(OndaSocialError::Unauthorized)
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
        msg!("leaf: {:?}", leaf.id());
        msg!("nonce: {:?}", leaf.nonce());

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
        msg!("post_count: {:?}", forum_config.post_count);

        Ok(())
    }
}

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
    /// CHECK: This account must be all zeros
    pub merkle_tree: UncheckedAccount<'info>,
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