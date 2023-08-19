
use anchor_lang::prelude::*;

#[error_code]
pub enum OndaSocialError {
  #[msg("Invalid uri.")]
  InvalidUri,
  #[msg("Title too long.")]
  TitleTooLong,
  #[msg("Insufficient post capacity.")]
  InsufficientPostCapacity,
  #[msg("Unauthorized.")]
  Unauthorized,
}