
use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCodes {
  #[msg("Member already exists.")]
  MemberAlreadyExists,
  #[msg("Member not found.")]
  MemberNotFound,
  #[msg("Unauthorized.")]
  Unauthorized,
}