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
    #[msg("Award claim not provided")]
    ClaimNotProvided,
    #[msg("Invalid claim")]
    InvalidClaim,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AwardClaims {
    award: Pubkey,
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
    pub metadata: AwardMetadata,
}   

#[account]
pub struct Award {
    /// The cost in lamports to mint a reward
    pub amount: u64,
    /// The amount which goes to the creator
    pub fee_basis_points: u16,
    /// The tree's authority
    pub authority: Pubkey,
    /// The award's treasury for fees
    pub treasury: Pubkey,
    /// The merkle tree used for minting cNFTs
    pub merkle_tree: Pubkey,
    /// The award's collection mint
    pub collection_mint: Pubkey,
    /// (optional) Requird signer for minting
    /// Needs to be provided if the uri is not provided
    pub additional_signer: Option<Pubkey>,
    /// The award metadata
    pub metadata: AwardMetadata,
    /// Gives claim to the matching award
    pub matching: Option<AwardClaims>,
}

impl Award {    
    pub const SIZE: usize = 8 + 
        8 + // amount
        2 + // fee_basis_points
        32 + // authority
        32 + // treasury
        32 + // merkle_tree
        32 + // collection_mint
        1 + 32 + // (optional) signer
        4 + MAX_NAME_LENGTH + 
        4 + MAX_SYMBOL_LENGTH + 
        4 + MAX_URI_LENGTH +
        1 + std::mem::size_of::<AwardClaims>(); // standard
}

#[account]
#[derive(Default)]
pub struct Claim {
    pub amount: u8,
}

impl Claim {
    pub const SIZE: usize = 8 + 1;
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
    pub matching_award: Option<Account<'info, Award>>,
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
    #[account(
        init_if_needed,
        seeds = [
            b"claim",
            award.matching.as_ref().unwrap().award.as_ref(),
            recipient.key().as_ref()
        ],
        space = Claim::SIZE,
        payer = payer,
        bump,
    )]
    pub claim: Option<Account<'info, Claim>>,
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

#[derive(Accounts)]
pub struct ClaimAward<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,
    /// (Optional) Additional signer if the award requires it
    pub additional_signer: Option<Signer<'info>>,
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        constraint = treasury.key.eq(&award.treasury) @ OndaAwardsError::InvalidTreasury
    )]
    pub award: Account<'info, Award>,
    #[account(
        mut,
        seeds = [
            b"claim",
            award.key().as_ref(),
            recipient.key().as_ref()
        ],
        bump,
    )]
    pub claim: Account<'info, Claim>,
    #[account(mut)]
    /// CHECK: not dangerous
    pub treasury: UncheckedAccount<'info>,
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

        award.matching = match &ctx.accounts.matching_award {
            Some(matching_award) => {
                require_keys_eq!(
                    matching_award.authority,
                    ctx.accounts.payer.key(),
                    OndaAwardsError::Unauthorized
                );

                Some(AwardClaims {
                    award: matching_award.key(),
                })
            },
            _ => None,
        };

        award.amount = args.amount;
        award.fee_basis_points = args.fee_basis_points;
        award.authority = ctx.accounts.payer.key();
        award.additional_signer = match &ctx.accounts.additional_signer {
            Some(signer) => Some(signer.key()),
            None => None,
        };
        award.treasury = ctx.accounts.treasury.key();
        award.collection_mint = ctx.accounts.collection_mint.key();
        award.merkle_tree = ctx.accounts.merkle_tree.key();
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
        let award = &ctx.accounts.award;
        let entry = &ctx.accounts.entry_id;
        let recipient = &ctx.accounts.recipient;
        let treasury = &ctx.accounts.treasury;
        let additional_signer = &ctx.accounts.additional_signer;

        if award.additional_signer.is_some() {
            require!(additional_signer.is_some(), OndaAwardsError::Unauthorized);
            require_keys_eq!(
                award.additional_signer.unwrap(),
                additional_signer.clone().unwrap().key(),
                OndaAwardsError::Unauthorized
            );
        }

        // Handle any claims
        if award.matching.is_some() {
            let wrapped_claim = &mut ctx.accounts.claim;

            match wrapped_claim {
                Some(claim) => {
                    claim.amount += 1;
                },
                None => {
                    return err!(OndaAwardsError::ClaimNotProvided);
                }
            }
        }


        // Payment
        let amount = u128::from(award.amount);
        let basis_points = award.fee_basis_points as u128;
        let fee = amount.checked_mul(basis_points).unwrap().checked_div(10_000).unwrap();
        let remaining_amount = amount.checked_sub(fee).unwrap();

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: treasury.to_account_info(),
                },
            ),
            fee as u64,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: recipient.to_account_info(),
                },
            ),
            remaining_amount as u64,
        )?;

        // Mint award
        let metadata_uri = match uri {
            Some(uri) => {
                if uri.len() > MAX_URI_LENGTH {
                    return err!(OndaAwardsError::InvalidUri);
                }

                if award.additional_signer.is_none() {
                    return err!(OndaAwardsError::Unauthorized);
                }

                uri
            },
            None => award.metadata.uri.clone(),
        };

        let bump = *ctx.bumps.get("award").unwrap();
        let seed = ctx.accounts.merkle_tree.clone().key();
        let signer_seeds = &[
            seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&signer_seeds[..]];
    
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum_program.to_account_info(),
            mpl_bubblegum::cpi::accounts::MintToCollectionV1 {
                tree_authority: ctx.accounts.tree_authority.to_account_info(), 
                leaf_owner: entry.to_account_info(),
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

    pub fn claim_award<'info>(ctx: Context<'_, '_, '_, 'info, ClaimAward<'info>>) -> Result<()> {
        let award = &ctx.accounts.award;
        let recipient = &ctx.accounts.recipient;
        let claim = &mut ctx.accounts.claim;

        if claim.amount == 0 {
            return err!(OndaAwardsError::ClaimNotProvided);
        }

        claim.amount -= 1;

        let bump = *ctx.bumps.get("award").unwrap();
        let seed = ctx.accounts.merkle_tree.clone().key();
        let signer_seeds = &[
            seed.as_ref(),
            &[bump],
        ];
        let signer_seeds = &[&signer_seeds[..]];
    
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum_program.to_account_info(),
            mpl_bubblegum::cpi::accounts::MintToCollectionV1 {
                tree_authority: ctx.accounts.tree_authority.to_account_info(), 
                leaf_owner: recipient.to_account_info(),
                leaf_delegate: award.to_account_info(),                
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                payer: ctx.accounts.recipient.to_account_info(),
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
                address: award.key(),
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
            uri: award.metadata.uri.clone(),
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
        })?;

        if claim.amount == 0 {
            claim.close(recipient.to_account_info())?;
        }

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