use anchor_lang::prelude::*;

#[account]
pub struct Collection {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub config: Config,
    pub reserved: [u8; 64],
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Debug)]
pub struct Config {
    pub loan_enabled: bool,
    pub loan_basis_points: u16,
    pub option_enabled: bool,
    pub option_basis_points: u16,
    pub rental_enabled: bool,
    pub rental_basis_points: u16, 
}

impl Collection {
    pub fn space() -> usize {
        8 +
        32 + // authority
        32 + // collection
        1 + 2 + 1 + 2 + 1 + 2 + // config
        64 + // reserved
        1 // bump
    }

    pub const PREFIX: &'static [u8] = b"collection";
}