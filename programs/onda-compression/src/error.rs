
use anchor_lang::prelude::*;

#[error_code]
pub enum OndaSocialError {
  #[msg("Invalid uri")]
  InvalidUri,
  #[msg("Title too long")]
  TitleTooLong,
  #[msg("Tag too long")]
  FlairTooLong,
  #[msg("Invalid flair")]
  InvalidFlair,
  #[msg("Insufficient post capacity")]
  InsufficientPostCapacity,
  #[msg("Invalid collection")]
  InvalidCollection,
  #[msg("Invalid method")]
  InvalidMethod,
  #[msg("Unauthorized")]
  Unauthorized,
}