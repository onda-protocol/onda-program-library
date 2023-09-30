use anchor_lang::prelude::*;
use spl_account_compression;
use mpl_bubblegum::program::Bubblegum;
use mpl_token_metadata::instruction::approve_collection_authority;
use onda_compression::state::LeafSchema;

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
    #[msg("Award amount too low for claim")]
    AwardAmountTooLowForClaim,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AwardClaims {
    award: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct CreateAwardArgs {
    pub amount: u64,
    pub fee_basis_points: u16,
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
    #[account(
        init,
        seeds = [merkle_tree.key().as_ref()],
        payer = payer,
        space = Award::SIZE,
        bump,
    )]
    pub award: Box<Account<'info, Award>>,
    /// CHECK: not dangerous
    pub matching_award: Option<Box<Account<'info, Award>>>,
    /// CHECK: not dangerous
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_metadata: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub collection_authority_record: UncheckedAccount<'info>,
    #[account(zero)]
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
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GiveAward<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
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
    /// CHECK: checked in cpi
    pub forum_merkle_tree: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: contrained by reward seeds
    pub merkle_tree: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub tree_authority: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_authority_record_pda: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    #[account(mut)]
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
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimAward<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        constraint = treasury.key.eq(&award.treasury) @ OndaAwardsError::InvalidTreasury
    )]
    pub award: Box<Account<'info, Award>>,
    #[account(
        mut,
        seeds = [
            b"claim",
            award.key().as_ref(),
            recipient.key().as_ref()
        ],
        bump,
    )]
    pub claim: Box<Account<'info, Claim>>,
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
    /// CHECK: checked in cpi
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    #[account(mut)]
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
        award.authority = ctx.accounts.payer.key();
        award.treasury = ctx.accounts.treasury.key();
        award.collection_mint = ctx.accounts.collection_mint.key();
        award.merkle_tree = ctx.accounts.merkle_tree.key();

        award.matching = match &ctx.accounts.matching_award {
            Some(matching_award) => {
                require_keys_eq!(
                    matching_award.authority,
                    ctx.accounts.payer.key(),
                    OndaAwardsError::Unauthorized
                );

                let claim_fee = ctx.accounts.rent.minimum_balance(Claim::SIZE);
                let (_fee, remaining_amount) = calculate_fee(award);

                if claim_fee > remaining_amount {
                    msg!("Award amount too low for claim rent exemption");
                    return err!(OndaAwardsError::AwardAmountTooLowForClaim);
                }

                Some(AwardClaims {
                    award: matching_award.key(),
                })
            },
            _ => None,
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
        root: [u8; 32],
        created_at: i64,
        edited_at: Option<i64>,
        data_hash: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let award = &ctx.accounts.award;
        let claim = &mut ctx.accounts.claim;
        let entry = &ctx.accounts.entry_id;
        let recipient = &ctx.accounts.recipient;
        let treasury = &ctx.accounts.treasury;

        // Handle any claims
        if award.matching.is_some() {
            match claim {
                Some(c) => {
                    c.amount += 1;
                },
                None => {
                    return err!(OndaAwardsError::ClaimNotProvided);
                }
            }
        }

        // Verify entry
        let cpi_ctx = CpiContext::new(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::VerifyLeaf {
                merkle_tree: ctx.accounts.forum_merkle_tree.to_account_info()
            }
        ).with_remaining_accounts(ctx.remaining_accounts.to_vec());
        let leaf = LeafSchema::new_v0(
            entry.key(),
            recipient.key(),
            created_at,
            edited_at,
            index as u64,
            data_hash,
        ).to_node();
        spl_account_compression::cpi::verify_leaf(
            cpi_ctx,
            root,
            leaf,
            index
        )?;

        // Payment
        let claim_fee = ctx.accounts.rent.minimum_balance(Claim::SIZE);
        let (fee, remaining_amount) = calculate_fee(award);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: treasury.to_account_info(),
                },
            ),
            fee,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: recipient.to_account_info(),
                },
            ),
            // Recipient gets the remaining amount when the claim is closed
            if claim.is_some() { 
                remaining_amount - claim_fee
            } else {
                remaining_amount
            },
        )?;

        // Mint award
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

        let metadata_account = &ctx.accounts.collection_metadata;
        let metadata = mpl_token_metadata::state::Metadata::deserialize(&mut metadata_account.data.borrow_mut().as_ref())?;

        mpl_bubblegum::cpi::mint_to_collection_v1(cpi_ctx, mpl_bubblegum::state::metaplex_adapter::MetadataArgs {
            name: metadata.data.name.clone(),
            symbol: metadata.data.symbol.clone(),
            uri: metadata.data.uri.clone(),
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

        let metadata_account = &ctx.accounts.collection_metadata;
        let metadata = mpl_token_metadata::state::Metadata::deserialize(&mut metadata_account.data.borrow_mut().as_ref())?;
    
        mpl_bubblegum::cpi::mint_to_collection_v1(cpi_ctx, mpl_bubblegum::state::metaplex_adapter::MetadataArgs {
            name: metadata.data.name.clone(),
            symbol: metadata.data.symbol.clone(),
            uri: metadata.data.uri.clone(),
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

pub fn calculate_fee(award: &Account<Award>) -> (u64, u64) {
    let amount = u128::from(award.amount);
    let basis_points = award.fee_basis_points as u128;
    let fee = amount.checked_mul(basis_points).unwrap().checked_div(10_000).unwrap();
    let remaining_amount = amount.checked_sub(fee).unwrap();

    (fee as u64, remaining_amount as u64)
}