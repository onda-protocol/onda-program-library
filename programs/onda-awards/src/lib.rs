use anchor_lang::prelude::*;
use gpl_session::{SessionError, SessionToken, session_auth_or, Session};
use mpl_bubblegum::{program::Bubblegum};
use mpl_token_metadata::{instruction::approve_collection_authority};

declare_id!("AwrdSLTcfNkVSARz8YoNYcVhknD7oxm7t3EqyYZ9bPK5");

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_SYMBOL_LENGTH: usize = 10;
pub const MAX_URI_LENGTH: usize = 200;

#[error_code]
pub enum OndaAwardsError {
    #[msg("Unauthorized.")]
    Unauthorized,
    #[msg("Numeric overflow.")]
    NumericOverflow,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AwardMetadata {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

#[account]
pub struct Award {
    /// The cost in lamports to mint a reward
    pub amount: u64,
    /// The tree's authority
    pub authority: Pubkey,
    /// The reward's collection mint
    pub collection_mint: Pubkey,
    /// The reward metadata
    pub metadata: AwardMetadata,
}

impl Award {    
    pub const SIZE: usize = 8 + 8 + 32 + 32 + 4 + MAX_NAME_LENGTH + 4 + MAX_SYMBOL_LENGTH + 4 + MAX_URI_LENGTH;
}

#[derive(Accounts)]
pub struct CreateAward<'info> {
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = Award::SIZE,
        bump,
    )]
    pub award: Account<'info, Award>,
    /// CHECK: should add check for collection authority
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_metadata: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub collection_authority_record: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub merkle_tree: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub tree_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: checked in cpi
    pub log_wrapper: UncheckedAccount<'info>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    /// CHECK: Checked in cpi
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub compression_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts, Session)]
pub struct GiveAward<'info> {
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
        seeds = [merkle_tree.key().as_ref()],
        bump,
    )]
    pub award: Account<'info, Award>,
    /// CHECK: This account is neither written to nor read from.
    pub leaf_owner: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: contrained by reward seeds
    pub merkle_tree: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub tree_authority: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_authority_record_pda: UncheckedAccount<'info>,
    /// CHECK: will fail if reward does not match
    pub collection_mint: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub collection_metadata: UncheckedAccount<'info>,
    /// CHECK: Checked in cpi
    pub edition_account: UncheckedAccount<'info>,
    /// CHECK: Checked in cpi
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: Checked in cpi
    pub bubblegum_signer: UncheckedAccount<'info>,
    /// CHECK: Checked in cpi
    pub compression_program: UncheckedAccount<'info>,
    /// CHECK: Checked in cpi
    pub token_metadata_program: UncheckedAccount<'info>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod onda_awards {
    use super::*;

    pub fn create_award(
        ctx: Context<CreateAward>,
        max_depth: u32,
        max_buffer_size: u32,
        metadata_args: AwardMetadata,
    ) -> Result<()> {
        let award = &mut ctx.accounts.award;

        // TODO: handle fees 
        award.amount = 0;
        award.authority = ctx.accounts.payer.key();
        award.collection_mint = ctx.accounts.collection_mint.key();
        award.metadata = AwardMetadata {
            name: puffed_out_string(&metadata_args.name, MAX_NAME_LENGTH),
            symbol: puffed_out_string(&metadata_args.symbol, MAX_SYMBOL_LENGTH),
            uri: puffed_out_string(&metadata_args.uri, MAX_URI_LENGTH),
        };

        let bump = *ctx.bumps.get("award").unwrap();
        let seed = ctx.accounts.merkle_tree.key();
        let signer_seeds = &[
            seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum_program.to_account_info(),
            mpl_bubblegum::cpi::accounts::CreateTree {
                tree_authority: ctx.accounts.tree_authority.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                tree_creator: award.to_account_info(),
                log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
                compression_program: ctx.accounts.compression_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer_seeds
        );

        mpl_bubblegum::cpi::create_tree(cpi_ctx, max_depth, max_buffer_size, None)?;

        anchor_lang::solana_program::program::invoke(
            &approve_collection_authority(
                ctx.accounts.token_metadata_program.key(),
                ctx.accounts.collection_authority_record.key(),
                award.key(),
                ctx.accounts.payer.key(),
                ctx.accounts.payer.key(),
                ctx.accounts.collection_metadata.key(),
                ctx.accounts.collection_mint.key(),
            ),
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.collection_authority_record.to_account_info(),
                award.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.collection_metadata.to_account_info(),
                ctx.accounts.collection_mint.to_account_info(),
            ],
        )?;

        Ok(())
    }

    #[session_auth_or(
        ctx.accounts.payer.key() == ctx.accounts.signer.key(),
        OndaAwardsError::Unauthorized
    )]
    pub fn give_award(ctx: Context<GiveAward>) -> Result<()> {
        let award = &ctx.accounts.award;

        let bump = *ctx.bumps.get("award").unwrap();
        let seed = ctx.accounts.merkle_tree.key();
        let signer_seeds = &[
            seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&signer_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum_program.to_account_info(),
            mpl_bubblegum::cpi::accounts::MintToCollectionV1 {
                tree_authority: ctx.accounts.tree_authority.to_account_info(), 
                leaf_owner: ctx.accounts.leaf_owner.to_account_info(),
                leaf_delegate: award.to_account_info(),                
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                payer: ctx.accounts.signer.to_account_info(),
                tree_delegate: award.to_account_info(),
                collection_authority: ctx.accounts.award.to_account_info(),
                collection_authority_record_pda: ctx.accounts.collection_authority_record_pda.to_account_info(),
                collection_mint: ctx.accounts.collection_mint.to_account_info(),
                collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
                edition_account: ctx.accounts.edition_account.to_account_info(),
                log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
                bubblegum_signer: ctx.accounts.bubblegum_signer.to_account_info(),
                compression_program: ctx.accounts.compression_program.to_account_info(),
                token_metadata_program: ctx.accounts.token_metadata_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer_seeds
        );

        let creators = vec![mpl_bubblegum::state::metaplex_adapter::Creator {
            address: ctx.accounts.payer.key(),
            verified: true,
            share: 100,
        }];

        mpl_bubblegum::cpi::mint_to_collection_v1(cpi_ctx, mpl_bubblegum::state::metaplex_adapter::MetadataArgs {
            name: award.metadata.name.clone(),
            symbol: award.metadata.symbol.clone(),
            uri: award.metadata.uri.clone(),
            seller_fee_basis_points: 0,
            primary_sale_happened: true,
            is_mutable: false,
            edition_nonce: None,
            token_standard: Some(mpl_bubblegum::state::metaplex_adapter::TokenStandard::NonFungible),
            collection: Some(mpl_bubblegum::state::metaplex_adapter::Collection {
                verified: false,
                key: ctx.accounts.collection_mint.key(),
            }),
            uses: None,
            token_program_version: mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion::Original,
            creators,
        })?;

        Ok(())
    }
}

pub fn puffed_out_string(s: &str, size: usize) -> String {
    let mut array_of_zeroes = vec![];
    let puff_amount = size - s.len();
    while array_of_zeroes.len() < puff_amount {
        array_of_zeroes.push(0u8);
    }
    s.to_owned() + std::str::from_utf8(&array_of_zeroes).unwrap()
}