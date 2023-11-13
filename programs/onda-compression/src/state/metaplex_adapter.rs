use anchor_lang::prelude::*;
use mpl_bubblegum;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenProgramVersion {
    Original,
    Token2022,
}

impl TokenProgramVersion {
    pub fn adapt(&self) -> mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion {
        match self {
            TokenProgramVersion::Original => {
                mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion::Original
            }
            TokenProgramVersion::Token2022 => {
                mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion::Token2022
            }
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    // In percentages, NOT basis points ;) Watch out!
    pub share: u8,
}

impl Creator {
    pub fn adapt(&self) -> mpl_bubblegum::state::metaplex_adapter::Creator {
        mpl_bubblegum::state::metaplex_adapter::Creator {
            address: self.address,
            verified: self.verified,
            share: self.share,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenStandard {
    NonFungible,        // This is a master edition
    FungibleAsset,      // A token with metadata that can also have attributes
    Fungible,           // A token with simple metadata
    NonFungibleEdition, // This is a limited edition
}

impl TokenStandard {
    pub fn adapt(&self) -> mpl_bubblegum::state::metaplex_adapter::TokenStandard {
        match self {
            TokenStandard::NonFungible => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::NonFungible
            }
            TokenStandard::FungibleAsset => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::FungibleAsset
            }
            TokenStandard::Fungible => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::Fungible
            }
            TokenStandard::NonFungibleEdition => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::NonFungibleEdition
            }
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum UseMethod {
    Burn,
    Multiple,
    Single,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Uses {
    // 17 bytes + Option byte
    pub use_method: UseMethod, //1
    pub remaining: u64,        //8
    pub total: u64,            //8
}

impl Uses {
    pub fn adapt(&self) -> mpl_bubblegum::state::metaplex_adapter::Uses {
        mpl_bubblegum::state::metaplex_adapter::Uses {
            use_method: match self.use_method {
                UseMethod::Burn => mpl_bubblegum::state::metaplex_adapter::UseMethod::Burn,
                UseMethod::Multiple => mpl_bubblegum::state::metaplex_adapter::UseMethod::Multiple,
                UseMethod::Single => mpl_bubblegum::state::metaplex_adapter::UseMethod::Single,
            },
            remaining: self.remaining,
            total: self.total,
        }
    }
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}

impl Collection {
    pub fn adapt(&self) -> mpl_bubblegum::state::metaplex_adapter::Collection {
        mpl_bubblegum::state::metaplex_adapter::Collection {
            verified: self.verified,
            key: self.key,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct MetadataArgs {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
    /// Royalty basis points that goes to creators in secondary sales (0-10000)
    pub seller_fee_basis_points: u16,
    // Immutable, once flipped, all sales of this metadata are considered secondary.
    pub primary_sale_happened: bool,
    // Whether or not the data struct is mutable, default is not
    pub is_mutable: bool,
    /// nonce for easy calculation of editions, if present
    pub edition_nonce: Option<u8>,
    /// Since we cannot easily change Metadata, we add the new DataV2 fields here at the end.
    pub token_standard: Option<TokenStandard>,
    /// Collection
    pub collection: Option<Collection>,
    /// Uses
    pub uses: Option<Uses>,
    pub token_program_version: TokenProgramVersion,
    pub creators: Vec<Creator>,
}
