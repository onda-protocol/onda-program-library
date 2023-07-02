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
    state::*,
};
pub mod error;
pub mod state;

declare_id!("ondaV4qqTUGbPR3m4XGi3YXf1NAXYCJEtuMKzVWBbSy");

pub const MAX_URI_LEN: usize = 128;

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
    #[account(
        associated_token::mint = mint,
        associated_token::authority = author,
    )]
    pub token_account: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// CHECK: constrained by seeds
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DeleteEntry<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    #[account(mut)]
    /// CHECK: constrained by seeds
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,

}

#[program]
pub mod onda_compression {
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

        match data.clone() {
            DataV1::TextPost { uri, .. } => {
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
            DataV1::ImagePost { uri, .. } => {
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
            DataV1::LinkPost { uri, .. } => {
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
            DataV1::VideoPost { uri, .. } => {
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
            DataV1::Comment { uri, .. } => {
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
        }

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
                require_keys_eq!(metadata_collection.key, address, OndaSocialError::Unauthorized);

                // Check the token account is owned by the author and has correct balance
                require_eq!(token_account.amount, 1, OndaSocialError::Unauthorized);
                require_eq!(token_account.mint, mint.key(), OndaSocialError::Unauthorized);
                require_keys_eq!(token_account.owner, author, OndaSocialError::Unauthorized);
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

    pub fn delete_entry<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteEntry<'info>>,
        root: [u8; 32],
        created_at: i64,
        edited_at: Option<i64>,
        data_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        msg!("root: {:?}", root);

        for proof in ctx.remaining_accounts.iter() {
            msg!("proof: {:?}", proof.key());
        }

        let author = &ctx.accounts.author;
        let entry_id = get_entry_id(&ctx.accounts.merkle_tree.key(), nonce);
        let previous_leaf = LeafSchema::new_v0(
            entry_id,
            author.key(),
            created_at,
            edited_at,
            nonce,
            data_hash,
        );

        let new_leaf = Node::default();

        replace_leaf(
            &ctx.accounts.merkle_tree.key(),
            *ctx.bumps.get("forum_config").unwrap(),
            &ctx.accounts.compression_program.to_account_info(),
            &ctx.accounts.forum_config.to_account_info(),
            &ctx.accounts.merkle_tree.to_account_info(),
            &ctx.accounts.log_wrapper.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf,
            index,
        )
    }
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

pub fn replace_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    compression_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_tree: &AccountInfo<'info>,
    log_wrapper: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    root_node: Node,
    previous_leaf: Node,
    new_leaf: Node,
    index: u32,
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
    )
    .with_remaining_accounts(remaining_accounts.to_vec());
    spl_account_compression::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf, new_leaf, index)
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