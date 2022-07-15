use {
  anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke_signed},
    },
  },
  mpl_token_metadata::{
    instruction::{freeze_delegated_account, thaw_delegated_account}
  },
};
use crate::state::{Loan};


pub struct FreezeParams<'a> {
  /// CHECK
  pub loan: AccountInfo<'a>,
  /// CHECK
  pub borrower: AccountInfo<'a>,
  /// CHECK
  pub deposit_token_account: AccountInfo<'a>,
  /// CHECK
  pub edition: AccountInfo<'a>,
  /// CHECK
  pub mint: AccountInfo<'a>,
}

pub fn freeze<'a>(bump: u8, params: FreezeParams<'a>) -> Result<()> {
  let FreezeParams {
      loan,
      borrower,
      deposit_token_account,
      edition,
      mint,
  } = params;
  
  let signer_bump = &[bump];
  let signer_seeds = &[&[
      Loan::PREFIX,
      mint.key.as_ref(),
      borrower.key.as_ref(),
      signer_bump
  ][..]];

  invoke_signed(
      &freeze_delegated_account(
          mpl_token_metadata::ID,
          loan.key(),
          deposit_token_account.key(),
          edition.key(),
          mint.key()
      ),
      &[
          loan,
          deposit_token_account.clone(),
          edition,
          mint
      ],
      signer_seeds
  )?;

  Ok(())
}

pub struct ThawParams<'a> {
  /// CHECK
  pub loan: AccountInfo<'a>,
  /// CHECK
  pub borrower: AccountInfo<'a>,
  /// CHECK
  pub deposit_token_account: AccountInfo<'a>,
  /// CHECK
  pub edition: AccountInfo<'a>,
  /// CHECK
  pub mint: AccountInfo<'a>,
}

pub fn thaw<'a>(bump: u8, params: ThawParams<'a>) -> Result<()> {
  let ThawParams {
      loan,
      borrower,
      deposit_token_account,
      edition,
      mint,
  } = params;
  
  let signer_bump = &[bump];
  let signer_seeds = &[&[
      Loan::PREFIX,
      mint.key.as_ref(),
      borrower.key.as_ref(),
      signer_bump
  ][..]];

  invoke_signed(
      &thaw_delegated_account(
          mpl_token_metadata::ID,
          loan.key(),
          deposit_token_account.key(),
          edition.key(),
          mint.key()
      ),
      &[
          loan,
          deposit_token_account.clone(),
          edition,
          mint
      ],
      signer_seeds
  )?;

  Ok(())
}