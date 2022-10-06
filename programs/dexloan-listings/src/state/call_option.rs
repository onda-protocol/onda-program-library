use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum CallOptionState {
    Listed,
    Active,
    Exercised,
}

#[account]
pub struct CallOption {
    /// Whether the option is active
    pub state: CallOptionState,
    /// The cost of the call option
    pub amount: u64,
    /// The issuer of the call option
    pub seller: Pubkey,
    /// The buyer of the call option
    pub buyer: Option<Pubkey>,
    /// Duration of the loan in seconds
    pub expiry: i64,
    /// The start date of the loan
    pub strike_price: u64,
    /// The mint of the token being used for collateral
    pub mint: Pubkey,
    /// (Optional) The mint of the spl-token mint
    pub token_mint: Option<Pubkey>,
    /// Misc
    pub bump: u8,
}

impl CallOption {
    pub fn space() -> usize {
        8 + // key
        1 + // state
        8 + // amount
        32 + // seller
        1 + 32 + // buyer
        8 + // expiry
        8 + // strike price
        32 + // mint
        32 + // token mint
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"call_option";
}

#[account]
pub struct CallOptionOffer {
    /// Duration of the loan in seconds
    pub expiry: i64,
    /// The start date of the loan
    pub strike_price: u64,
    /// The cost of the call option
    pub amount: u64,
    /// misc
    pub bump: u8,
}

impl CallOptionOffer {
    pub fn space() -> usize {
        8 + // key
        8 + // expiry
        8 + // strike_price
        8 + // amount
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"call_option";
}