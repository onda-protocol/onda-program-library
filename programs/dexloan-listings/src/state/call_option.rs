use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::*;

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
    pub fn init_state(mut self, amount: u64, strike_price: u64, expiry: i64) {
        self.state = CallOptionState::Listed;
        self.amount = amount;
        self.strike_price = strike_price;
        self.expiry = expiry;
    }

    pub fn set_active(mut self, unix_timestamp: i64) {
        require_eq!(self.state, CallOptionState::Listed, DexloanError::InvalidState);
        require!(self.seller != SYSTEM_ACCOUNT, DexloanError::InvalidState);
        require!(self.expiry > unix_timestamp, DexloanError::InvalidExpiry);
        require!(self.buyer.is_some(), DexloanError::InvalidState);
        require!(self.amount > 0, DexloanError::InvalidState);
        require!(self.strike_price > 0, DexloanError::InvalidState);

        self.state = CallOptionState::Active;
    } 

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
pub struct CallOptionBid {
    pub id: u8,
    /// The buyer making the offer
    pub buyer: Pubkey,
    /// Duration of the loan in seconds
    pub expiry: i64,
    /// The start date of the loan
    pub strike_price: u64,
    /// The cost of the call option
    pub amount: u64,
    /// The collection
    pub collection: Pubkey,
    /// misc
    pub bump: u8,
    pub escrow_bump: u8,
}

impl CallOptionBid {
    pub fn space() -> usize {
        8 + // key
        1 + // id
        32 + // buyer
        8 + // expiry
        8 + // strike_price
        8 + // amount
        32 + // collection
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"call_option_bid";
    pub const VAULT_PREFIX: &'static [u8] = b"call_option_bid_vault";
}