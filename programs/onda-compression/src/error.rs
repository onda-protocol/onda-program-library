
use anchor_lang::prelude::*;

#[error_code]
pub enum OndaSocialError {
  #[msg("Insufficient post capacity.")]
  InsufficientPostCapacity,
  #[msg("Unauthorized.")]
  Unauthorized,
}