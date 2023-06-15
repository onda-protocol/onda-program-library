
use anchor_lang::prelude::*;

#[error_code]
pub enum OndaSocialError {
  #[msg("Invalid uri.")]
  InvalidUri,
  #[msg("Insufficient post capacity.")]
  InsufficientPostCapacity,
  #[msg("Unauthorized.")]
  Unauthorized,
}