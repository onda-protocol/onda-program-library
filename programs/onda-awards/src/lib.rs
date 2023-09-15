use anchor_lang::prelude::*;
use mpl_bubblegum::program::Bubblegum;
use mpl_token_metadata::instruction::approve_collection_authority;

declare_id!("AwrdSLTcfNkVSARz8YoNYcVhknD7oxm7t3EqyYZ9bPK5");

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_SYMBOL_LENGTH: usize = 10;
pub const MAX_URI_LENGTH: usize = 200;
pub const SELLER_FEE_BASIS_POINTS: usize = 500;

#[error_code]
pub enum OndaAwardsError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Numeric overflow")]
    NumericOverflow,
    #[msg("Invalid uri")]
    InvalidUri,
    #[msg("Invalid args")]
    InvalidArgs,
    #[msg("Invalid treasury")]
    InvalidTreasury,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum AwardStandard {
    Single,
    /// Awardee receives a duplicate of the award
    Matching,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AwardMetadata {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    /// If the uri is not provided it must be passed as an argument when creating the award
    /// Along with the specified signer
    pub uri: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct CreateAwardArgs {
    pub amount: u64,
    pub fee_basis_points: u16,
    pub standard: AwardStandard,
    pub metadata: AwardMetadata,
}   

#[account]
pub struct Award {
    pub standard: AwardStandard,
    /// The cost in lamports to mint a reward
    pub amount: u64,
    /// The amount which goes to the creator
    pub fee_basis_points: u16,
    /// The tree's authority
    pub authority: Pubkey,
    /// The award's treasury for fees
    pub treasury: Pubkey,
    /// The award's collection mint
    pub collection_mint: Pubkey,
    /// (optional) Requird signer for minting
    /// Needs to be provided if the uri is not provided
    pub additional_signer: Option<Pubkey>,
    /// The award metadata
    pub metadata: AwardMetadata,
}

impl Award {    
    pub const SIZE: usize = 8 + 
        1 + // standard
        8 + // amount
        2 + // fee_basis_points
        32 + // authority
        32 + // treasury
        32 + // collection_mint
        1 + 32 + // (optional) signer
        4 + MAX_NAME_LENGTH + 
        4 + MAX_SYMBOL_LENGTH + 
        4 + MAX_URI_LENGTH;
}

#[derive(Accounts)]
pub struct CreateAward<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: not dangerous
    pub additional_signer: Option<UncheckedAccount<'info>>,
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = Award::SIZE,
        bump,
    )]
    pub award: Account<'info, Award>,
    /// CHECK: not dangerous
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: payer must have collection authority
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
    /// CHECK: checked in cpi
    pub log_wrapper: UncheckedAccount<'info>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    /// CHECK: Checked in cpi
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub compression_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GiveAward<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// (Optional) Additional signer if the award requires it
    pub additional_signer: Option<Signer<'info>>,
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        constraint = treasury.key.eq(&award.treasury) @ OndaAwardsError::InvalidTreasury
    )]
    pub award: Account<'info, Award>,
    #[account(mut)]
    /// CHECK: not dangerous
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is neither written to nor read from.
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub entry_id: UncheckedAccount<'info>,
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
        args: CreateAwardArgs,
    ) -> Result<()> {
        let award = &mut ctx.accounts.award;

        if args.fee_basis_points > 10_000 {
            return err!(OndaAwardsError::InvalidArgs);
        }

        award.amount = args.amount;
        award.fee_basis_points = args.fee_basis_points;
        award.standard = args.standard;
        award.authority = ctx.accounts.payer.key();
        award.additional_signer = match &ctx.accounts.additional_signer {
            Some(signer) => Some(signer.key()),
            None => None,
        };
        award.treasury = ctx.accounts.treasury.key();
        award.collection_mint = ctx.accounts.collection_mint.key();
        award.metadata = AwardMetadata {
            name: puffed_out_string(&args.metadata.name, MAX_NAME_LENGTH),
            symbol: puffed_out_string(&args.metadata.symbol, MAX_SYMBOL_LENGTH),
            uri: puffed_out_string(&args.metadata.uri, MAX_URI_LENGTH),
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

    pub fn give_award<'info>(
        ctx: Context<'_, '_, '_, 'info, GiveAward<'info>>,
        uri: Option<String>
    ) -> Result<()> {
        let award = &ctx.accounts.award.clone();
        let entry = &ctx.accounts.entry_id.clone();
        let recipient = &ctx.accounts.recipient.clone();
        let bump = *ctx.bumps.get("award").unwrap();
        let seed = ctx.accounts.merkle_tree.clone().key();

        process_mint(
            &ctx, 
            award.metadata.uri.clone(),
            &entry.to_account_info(),
            &seed, 
            bump
        )?;

        if award.standard == AwardStandard::Matching {
            let metadata_uri = match uri {
                Some(uri) => uri,
                None => award.metadata.uri.clone(),
            };

            process_mint(
                &ctx, 
                metadata_uri.clone(),
                &recipient.to_account_info(),
                &seed, 
                bump
            )?;
        }

        let amount = u128::from(award.amount);
        let basis_points = award.fee_basis_points as u128;
        let fee = amount.checked_mul(basis_points).unwrap().checked_div(10_000).unwrap();
        let remaining_amount = amount.checked_sub(fee).unwrap();

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            fee as u64,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                },
            ),
            remaining_amount as u64,
        )?;

        Ok(())
    }
}

pub fn process_mint<'info>(
    ctx: &Context<'_, '_, '_, 'info, GiveAward<'info>>,
    metadata_uri: String,
    leaf_owner: &AccountInfo<'info>,
    seed: &Pubkey,
    bump: u8,
) -> Result<()> {
    let award = &ctx.accounts.award;
    // let bump = *ctx.bumps.get("award").unwrap();
    // let seed = ctx.accounts.merkle_tree.key();
    let signer_seeds = &[
        seed.as_ref(),
        &[bump],
    ];
    let signer_seeds = &[&signer_seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.bubblegum_program.to_account_info(),
        mpl_bubblegum::cpi::accounts::MintToCollectionV1 {
            tree_authority: ctx.accounts.tree_authority.to_account_info(), 
            leaf_owner: leaf_owner.to_account_info(),
            leaf_delegate: award.to_account_info(),                
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
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

    let creators = vec![
        mpl_bubblegum::state::metaplex_adapter::Creator {
            address: ctx.accounts.payer.key(),
            verified: true,
            share: 0,
        }, 
        mpl_bubblegum::state::metaplex_adapter::Creator {
            address: ctx.accounts.recipient.key(),
            verified: false,
            share: 100,
        }
    ];

    mpl_bubblegum::cpi::mint_to_collection_v1(cpi_ctx, mpl_bubblegum::state::metaplex_adapter::MetadataArgs {
        name: award.metadata.name.clone(),
        symbol: award.metadata.symbol.clone(),
        uri: metadata_uri.clone(),
        seller_fee_basis_points: SELLER_FEE_BASIS_POINTS as u16,
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
    })
}

pub fn puffed_out_string(s: &str, size: usize) -> String {
    let mut array_of_zeroes = vec![];
    let puff_amount = size - s.len();
    while array_of_zeroes.len() < puff_amount {
        array_of_zeroes.push(0u8);
    }
    s.to_owned() + std::str::from_utf8(&array_of_zeroes).unwrap()
}