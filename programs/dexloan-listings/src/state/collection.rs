use anchor_lang::prelude::*;

#[account]
pub struct Collection {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub reserved: [u8; 128],
    pub bump: u8,
}

impl Collection {
    pub fn space() -> usize {
        8 +
        32 + // authority
        32 + // collection
        128 + // reserved
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"collection";
}