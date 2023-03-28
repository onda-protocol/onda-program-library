use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AccountState {
  pub loan: bool,
  pub call_option: bool,
  pub rental: bool,
}

#[account]
pub struct TokenManager {
    pub authority: Option<Pubkey>,
    pub accounts: AccountState,
    /// Misc
    pub bump: u8,
}

impl TokenManager {
    pub const PREFIX: &'static [u8] = b"token_manager";
    pub const ESCROW_PREFIX: &'static [u8] = b"escrow";

    pub fn space() -> usize {
      8 + // key
      (1 + 32) + // authority
      (1 * 3) + // account state
      1 // bump
  }
}