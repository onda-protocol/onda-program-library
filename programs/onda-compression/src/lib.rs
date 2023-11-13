use url::Url;
use anchor_lang::{
    prelude::*,
    solana_program::keccak,
};
use anchor_spl::token::{Mint, TokenAccount};
use spl_account_compression::{
    program::SplAccountCompression, wrap_application_data_v1, Node, Noop,
};
use mpl_bubblegum::{state::leaf_schema, utils::get_asset_id, hash_metadata};
use mpl_token_metadata::{
    state::Metadata,
    pda::find_metadata_account
};
use gpl_session::{SessionError, SessionToken, session_auth_or, Session};

use crate::{
    error::OndaSocialError,
    state::{*, metaplex_adapter::*},
};
pub mod error;
pub mod state;

declare_id!("ondaTPaRbk5xRJiqje7DS8n6nFu7Hg6jvKthXNemsHg");

pub const MAX_TITLE_LEN: usize = 300;
pub const MAX_URI_LEN: usize = 128;
pub const MAX_FLAIR_LEN: usize = 42;

#[derive(Accounts)]
#[instruction(max_depth: u32, max_buffer_size: u32, flair: Vec<String>, gate: Option<Vec<Gate>>)]
pub struct InitForum<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = ForumConfig::get_size(flair, gate),
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
pub struct SetAdmin<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: new admin
    pub new_admin: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        bump,
        constraint = forum_config.admin == *admin.key @OndaSocialError::Unauthorized,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    /// CHECK: forum config
    pub merkle_tree: UncheckedAccount<'info>,
}

#[derive(Accounts, Session)]
pub struct AddEntry<'info> {
    /// CHECK: session auth
    pub author: UncheckedAccount<'info>,
    #[session(
        // The ephemeral keypair signing the transaction
        signer = signer,
        // The authority of the user account which must have created the session
        authority = author.key()
    )]
    // Session Tokens are passed as optional accounts
    pub session_token: Option<Account<'info, SessionToken>>,
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: check is signer
    pub additional_signer: Option<AccountInfo<'info>>,
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
    /// CHECK: verified in merkle tree
    pub nft_merkle_tree: Option<UncheckedAccount<'info>>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddEntryWithCompressedNft<'info> {
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
    /// CHECK: verified in merkle tree
    pub nft_merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DeleteEntry<'info> {
    /// CHECK: matches post author
    pub author: UncheckedAccount<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
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
        flair: Vec<String>,
        gate: Option<Vec<Gate>>,
    ) -> Result<()> {
        let forum_config = &mut ctx.accounts.forum_config;
        let merkle_tree = &ctx.accounts.merkle_tree;
        let seed = merkle_tree.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("forum_config").unwrap()]];
        let wrapper = &ctx.accounts.log_wrapper;
        
        forum_config.set_inner(ForumConfig {
            admin: ctx.accounts.payer.key(),
            total_capacity: 1 << max_depth,
            post_count: 0,
            flair,
            gate: gate.unwrap_or(vec![]),
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

    pub fn set_admin(ctx: Context<SetAdmin>) -> Result<()> {
        let forum_config = &mut ctx.accounts.forum_config;
        forum_config.set_admin(ctx.accounts.new_admin.key());
        Ok(())
    }

    #[session_auth_or(
        ctx.accounts.author.key() == ctx.accounts.signer.key(),
        OndaSocialError::Unauthorized
    )]
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
            DataV1::TextPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::ImagePost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::LinkPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::VideoPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::Comment { uri, .. } => {
                require!(is_valid_url(&uri), OndaSocialError::InvalidUri);
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
        }

        // Check if user is allowed to add an entry to this forum
        let gates = forum_config.gate.clone();
        let operation_results = gates.iter().map(|gate| {
            let addresses = gate.address.clone();
            let mut operation = OperationResult {
                operator: gate.operator.clone(),
                result: false,
            };

            match gate.rule_type {
                Rule::Token => {
                    if mint.is_some() && token_account.is_some() {
                        let mint = mint.clone().unwrap();
                        let token_account = token_account.clone().unwrap();
    
                        for address in addresses {
                            let is_valid = is_valid_token(
                                &address,
                                &author,
                                &mint,
                                &token_account,
                                gate.amount
                            );

                            if is_valid {
                                operation.result = true;
                                break;
                            }
                        }
                    }

                },
                Rule::Nft => {
                    if mint.is_some() && token_account.is_some() &&  metadata.is_some() {
                        let mint = mint.clone().unwrap();
                        let token_account = token_account.clone().unwrap();
                        let metadata_info = metadata.clone().unwrap();
    
                        for address in addresses {
                            let is_valid = is_valid_token(
                                &address,
                                &author,
                                &mint,
                                &token_account,
                                gate.amount
                            );
    
                            if is_valid == false {
                                break;
                            }
    
                            let is_valid = is_valid_nft(
                                &address,
                                &mint,
                                &metadata_info
                            );
    
                            if is_valid {
                                operation.result = true;
                                break;
                            }
                        }
                    }
                },
                Rule::CompressedNft => {
                    msg!("Use add_entry_with_compressed_nft instruction for compressed nft gating");
                    operation.result = false;
                },
                Rule::AdditionalSigner => {
                    if ctx.accounts.additional_signer.is_some() {
                        let additional_signer = &ctx.accounts.additional_signer.clone().unwrap();
                        
                        if additional_signer.is_signer == false {
                            operation.result = false;
                        } else {
                            for address in addresses {
                                if additional_signer.key().eq(&address) {
                                    match gate.operator {
                                        Operator::Not => {
                                            operation.result = false;
                                            break;
                                        },
                                        _ => {
                                            operation.result = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            operation
        }).collect::<Vec<OperationResult>>();
                
        let allow_access = evaluate_operations(operation_results);
        
        if allow_access == false {
            return err!(OndaSocialError::Unauthorized);
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

    pub fn add_entry_with_compressed_nft<'info>(
        ctx: Context<'_, '_, '_, 'info, AddEntryWithCompressedNft<'info>>,
        data: DataV1,
        root: [u8; 32],
        index: u32,
        nonce: u64,
        creator_hash: [u8; 32],
        metadata_args: MetadataArgs,
    ) -> Result<()> {
        let author =  ctx.accounts.author.key();
        let forum_config = &mut ctx.accounts.forum_config;
        let forum_config_bump = *ctx.bumps.get("forum_config").unwrap();
        let merkle_tree = &ctx.accounts.merkle_tree;
        let log_wrapper = &ctx.accounts.log_wrapper;
        let compression_program = &ctx.accounts.compression_program;

        match data.clone() {
            DataV1::TextPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::ImagePost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::LinkPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::VideoPost { title, uri, flair, .. } => {
                validate_flair(&forum_config, &flair)?;
                validate_post_schema(&title, &uri)?;
            },
            DataV1::Comment { uri, .. } => {
                require!(is_valid_url(&uri), OndaSocialError::InvalidUri);
                require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
            },
        }

        // Check if cNFT is valid gate
        if metadata_args.collection.is_none() {
            return err!(OndaSocialError::InvalidCollection);
        }

        let collection = metadata_args.collection.clone().unwrap();
        let gates = forum_config.gate.clone();
        let result = gates.iter().any(|gate| {
            gate.address.iter().any(|a| a.eq(&collection.key)) &&
            gate.rule_type == Rule::CompressedNft
        });

        if result == false {
            return err!(OndaSocialError::Unauthorized);
        }

        let cpi_ctx = CpiContext::new(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::VerifyLeaf {
                merkle_tree: ctx.accounts.nft_merkle_tree.to_account_info()
            }
        ).with_remaining_accounts(ctx.remaining_accounts.to_vec());
        let asset_id = get_asset_id(&merkle_tree.key(), nonce);
 
        let token_standard = if metadata_args.token_standard.is_some() {
            Some(metadata_args.token_standard.unwrap().adapt())
        } else {
            None
        };
        let collection = if metadata_args.collection.is_some() {
            Some(metadata_args.collection.unwrap().adapt())
        } else {
            None
        };
        let uses = if metadata_args.uses.is_some() {
            Some(metadata_args.uses.unwrap().adapt())
        } else {
            None
        };
        let creators = metadata_args.creators.iter().map(|c| c.adapt()).collect::<Vec<_>>();
        let data_hash = hash_metadata(&mpl_bubblegum::state::metaplex_adapter::MetadataArgs {
            name: metadata_args.name,
            symbol: metadata_args.symbol,
            uri: metadata_args.uri,
            seller_fee_basis_points: metadata_args.seller_fee_basis_points,
            primary_sale_happened: metadata_args.primary_sale_happened,
            is_mutable: metadata_args.is_mutable,
            edition_nonce: metadata_args.edition_nonce,
            token_standard: token_standard,
            collection: collection,
            uses: uses,
            token_program_version: metadata_args.token_program_version.adapt(),
            creators: creators,
        })?;
        let leaf = leaf_schema::LeafSchema::new_v0(
            asset_id,
            author.key(),
            author.key(),
            nonce,
            data_hash,
            creator_hash,
        ).to_node();
        spl_account_compression::cpi::verify_leaf(
            cpi_ctx,
            root,
            leaf,
            index
        )?;
        
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
        let forum_config = &mut ctx.accounts.forum_config;
        let signer = &ctx.accounts.signer;
        let author = &ctx.accounts.author;

        if signer.key().eq(&author.key()) == false && forum_config.admin.eq(&signer.key()) == false {
            return err!(OndaSocialError::Unauthorized);

        }

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

pub fn is_valid_url(input: &str) -> bool {
    match Url::parse(input) {
        Ok(_) => true,  // The URL is valid.
        Err(_) => false, // The URL is not valid.
    }
}

pub fn is_valid_token(
    address: &Pubkey,
    owner: &Pubkey,
    mint: &Account<Mint>,
    token_account: &Account<TokenAccount>,
    amount: u64,
) -> bool {
    if mint.key().eq(&address) == false {
        return false;
    }
    if token_account.mint.eq(&mint.key()) == false {
        return false;
    }
    if token_account.owner.eq(&owner) == false {
        return false;
    }
    if token_account.amount.ge(&1) == false {
        return false;
    }
    if amount < token_account.amount {
        return false;
    }
    true
} 

pub fn is_valid_nft(
    address: &Pubkey,
    mint: &Account<Mint>,
    metadata_info: &AccountInfo,
) -> bool {
    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    ).unwrap();

    // Check the metadata address is valid pda for this mint
    let (metadata_pda, _) = find_metadata_account(
        &mint.key()
    );

    if metadata_pda.eq(&metadata_info.key()) == false {
        return false
    }

    // Check the metadata is verified for this collection
    let is_valid_collection = match metadata.collection {
        Some(collection) => collection.verified == true && collection.key.eq(&address),
        None => false,
    };

    is_valid_collection
}

pub fn validate_flair(config: &ForumConfig, flair: &Option<String>) -> Result<bool> {
    if flair.is_none() {
        return Ok(true);
    }

    let flair = flair.clone().unwrap();
    require!(flair.len() <= MAX_FLAIR_LEN, OndaSocialError::FlairTooLong);
    require!(config.flair.iter().find(|name| **name == flair).is_some(), OndaSocialError::InvalidFlair);
    Ok(true)
}

pub fn validate_post_schema(title: &str, uri: &str) -> Result<bool> {
    require!(is_valid_url(&uri), OndaSocialError::InvalidUri);
    require_gte!(MAX_URI_LEN, uri.len(), OndaSocialError::InvalidUri);
    require_gte!(MAX_TITLE_LEN, title.len(), OndaSocialError::TitleTooLong);
    Ok(true)
}

pub fn evaluate_operations(operations: Vec<OperationResult>) -> bool {
    let mut overall_result = false;
    let mut or_case_result = false;

    if operations.len() == 0 {
        return true;
    }
    
    for op in operations {
        match op.operator {
            Operator::And => {
                overall_result &= op.result;
                or_case_result &= op.result;
            }
            Operator::Or => {
                if op.result {
                    or_case_result = true;
                }
                overall_result |= or_case_result;
            },
            Operator::Not => {
                if op.result == false {
                    overall_result = false;
                    break;
                }
            }
        }
    }
    
    overall_result
}