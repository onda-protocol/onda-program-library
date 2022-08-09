use {
  std::{slice::Iter},
  anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke, invoke_signed},
    },
  },
  mpl_token_metadata::{
    instruction::{freeze_delegated_account, thaw_delegated_account}
  },
  metaplex_token_metadata::state::{Metadata}
};
use crate::state::{Hire, TokenManager};
use crate::error::*;

pub struct FreezeParams<'a, 'b> {
  /// CHECK
  pub delegate: AccountInfo<'a>,
  /// CHECK
  pub token_account: AccountInfo<'a>,
  /// CHECK
  pub edition: AccountInfo<'a>,
  /// CHECK
  pub mint: AccountInfo<'a>,
  pub signer_seeds: &'b [&'b [&'b [u8]]]
}

pub fn freeze<'a, 'b>(params: FreezeParams<'a, 'b>) -> Result<()> {
  let FreezeParams {
      delegate,
      token_account,
      edition,
      mint,
      signer_seeds
  } = params;

  invoke_signed(
      &freeze_delegated_account(
          mpl_token_metadata::ID,
          delegate.key(),
          token_account.key(),
          edition.key(),
          mint.key()
      ),
      &[
          delegate,
          token_account.clone(),
          edition,
          mint
      ],
      signer_seeds
  )?;

  Ok(())
}

pub fn thaw<'a, 'b>(params: FreezeParams<'a, 'b>) -> Result<()> {
  let FreezeParams {
      delegate,
      token_account,
      edition,
      mint,
      signer_seeds,
  } = params;

  invoke_signed(
      &thaw_delegated_account(
          mpl_token_metadata::ID,
          delegate.key(),
          token_account.key(),
          edition.key(),
          mint.key()
      ),
      &[
          delegate,
          token_account.clone(),
          edition,
          mint
      ],
      signer_seeds
  )?;

  Ok(())
}

pub fn delegate_and_freeze_token_account<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    token_program: AccountInfo<'info>,
    token_account: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    edition: AccountInfo<'info>,
) -> Result<()> {    
    anchor_spl::token::approve(
        CpiContext::new(
            token_program,
            anchor_spl::token::Approve {
                to: token_account.clone(),
                delegate: token_manager.to_account_info(),
                authority: authority.clone(),
            }
        ),
        1
    )?;

    let mint_pubkey = mint.key();
    let issuer_pubkey = authority.key();
    let signer_bump = &[token_manager.bump];
    let signer_seeds = &[&[
        TokenManager::PREFIX,
        mint_pubkey.as_ref(),
        issuer_pubkey.as_ref(),
        signer_bump
    ][..]];

    freeze(
        FreezeParams {
            delegate: token_manager.to_account_info(),
            token_account,
            edition,
            mint,
            signer_seeds: signer_seeds
        }
    )?;

    Ok(())
}

pub fn thaw_token_account<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    token_account: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    edition: AccountInfo<'info>,
) -> Result<()> {
    let mint_pubkey = mint.key();
    let issuer_pubkey = authority.key();
    let signer_bump = &[token_manager.bump];
    let signer_seeds = &[&[
        TokenManager::PREFIX,
        mint_pubkey.as_ref(),
        issuer_pubkey.as_ref(),
        signer_bump
    ][..]];
  
    thaw(
        FreezeParams {
            delegate: token_manager.to_account_info(),
            token_account,
            edition,
            mint,
            signer_seeds: signer_seeds
        }
    )?;

    Ok(())
}

pub fn thaw_and_revoke_token_account<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    token_program: AccountInfo<'info>,
    token_account: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    edition: AccountInfo<'info>,
) -> Result<()> {
    thaw_token_account(
        token_manager,
        token_account.clone(),
        authority.clone(),
        mint,
        edition,
    )?;

    anchor_spl::token::revoke(
        CpiContext::new(
            token_program,
            anchor_spl::token::Revoke {
                source: token_account,
                authority,
            }
        )
    )?;

    Ok(())
}

pub fn thaw_and_transfer_from_token_account<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    token_program: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    from_token_account: AccountInfo<'info>,
    to_token_account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    edition: AccountInfo<'info>,
) -> Result<()> {
    let mint_pubkey = mint.key();
    let issuer_pubkey = authority.key();
    let signer_bump = &[token_manager.bump];
    let signer_seeds = &[&[
        TokenManager::PREFIX,
        mint_pubkey.as_ref(),
        issuer_pubkey.as_ref(),
        signer_bump
    ][..]];
  
    thaw(
        FreezeParams {
            delegate: token_manager.to_account_info(),
            token_account: from_token_account.clone(),
            edition,
            mint,
            signer_seeds: signer_seeds
        }
    )?;

    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: from_token_account,
                to: to_token_account,
                authority: token_manager.to_account_info(),
            },
            signer_seeds
        ),
        1
    )?;

    Ok(())
}

pub fn withdraw_from_escrow_balance<'info>(
    hire: &mut Account<'info, Hire>,
    lender: AccountInfo<'info>,
    unix_timestamp: i64,
) -> Result<u64> {
    require_keys_eq!(lender.key(), hire.lender);

    let start = hire.current_start.unwrap();
    let end = hire.current_expiry.unwrap();
    let pro_rata_amount = ((unix_timestamp - start) / (end - start)).checked_mul(hire.escrow_balance as i64).ok_or(DexloanError::NumericalOverflow).unwrap();

    msg!("Withdrawing {} from escrow balance ", pro_rata_amount);

    let signer_bump = &[hire.bump];
    let signer_seeds = &[&[
        Hire::PREFIX,
        hire.mint.as_ref(),
        hire.lender.as_ref(),
        signer_bump
    ][..]];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::system_instruction::transfer(
            &hire.key(),
            &hire.lender,
            pro_rata_amount as u64,
        ),
        &[
            hire.to_account_info(),
            lender,
        ],
        signer_seeds
    )?;

    let remaining_escrow_balance = hire.escrow_balance - pro_rata_amount as u64;

    Ok(remaining_escrow_balance)
}

// If a call option is exercised or a loan repossessed while a hire is active
// Then any unearned balance must be paid back to the hire's borrower
pub fn settle_hire_escrow_balance<'info>(
    hire: &mut Account<'info, Hire>,
    borrower: AccountInfo<'info>,
    lender: AccountInfo<'info>,
    unix_timestamp: i64,
) -> Result<()> {
    require_keys_eq!(borrower.key(), hire.borrower.unwrap());

    let remaining_escrow_balance = withdraw_from_escrow_balance(
        hire,
        lender,
        unix_timestamp,
    )?;

    msg!("Returning {} to borrower from escrow balance", remaining_escrow_balance);

    let signer_bump = &[hire.bump];
    let signer_seeds = &[&[
        Hire::PREFIX,
        hire.mint.as_ref(),
        hire.lender.as_ref(),
        signer_bump
    ][..]];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::system_instruction::transfer(
            &hire.key(),
            &hire.borrower.unwrap(),
            remaining_escrow_balance,
        ),
        &[
            hire.to_account_info(),
            borrower,
        ],
        signer_seeds
    )?;

    Ok(())
}

pub fn assert_metadata_valid<'a>(
    metadata: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
  ) -> Result<()> {
    let (key, _) = mpl_token_metadata::pda::find_metadata_account(
      &mint.key()
    );
  
    if key != metadata.to_account_info().key() {
      return err!(DexloanError::DerivedKeyInvalid);
    }
  
    if metadata.data_is_empty() {
      return err!(DexloanError::MetadataDoesntExist);
    }
  
    Ok(())
}
  
pub fn calculate_fee_from_basis_points(
    amount: u128,
    basis_points: u128,
) -> Result<u64> {
    let total_fee = basis_points.checked_mul(amount)
        .ok_or(DexloanError::NumericalOverflow)?
        .checked_div(10_000)
        .ok_or(DexloanError::NumericalOverflow)? as u64;
    
    Ok(total_fee)
}

pub fn pay_creator_fees<'a>(
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
    amount: u64,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &AccountInfo<'a>,
) -> Result<u64> {
    let metadata = Metadata::from_account_info(metadata_info)?;

    if metadata.mint != mint.key() {
        return  err!(DexloanError::InvalidMint);
    }

    assert_metadata_valid(
        &metadata_info,
        &mint
    )?;

    let fees = metadata.data.seller_fee_basis_points;
    let total_fee = calculate_fee_from_basis_points(amount as u128, fees as u128)?;
    let mut remaining_fee = total_fee;
    let remaining_amount = amount
            .checked_sub(total_fee)
            .ok_or(DexloanError::NumericalOverflow)?;

    msg!("Paying {} lamports in royalties", total_fee);
        
    match metadata.data.creators {
        Some(creators) => {
            for creator in creators {
                let pct = creator.share as u128;
                let creator_fee = pct.checked_mul(total_fee as u128)
                        .ok_or(DexloanError::NumericalOverflow)?
                        .checked_div(100)
                        .ok_or(DexloanError::NumericalOverflow)? as u64;
                remaining_fee = remaining_fee
                        .checked_sub(creator_fee)
                        .ok_or(DexloanError::NumericalOverflow)?;

                let current_creator_info = next_account_info(remaining_accounts)?;

                if creator_fee > 0 {
                    invoke(
                        &anchor_lang::solana_program::system_instruction::transfer(
                            &fee_payer.key(),
                            &current_creator_info.key(),
                            creator_fee,
                        ),
                        &[
                            current_creator_info.to_account_info(),
                            fee_payer.to_account_info(),
                        ]
                    )?;
                }
            }
        }
        None => {
            msg!("No creators found in metadata");
        }
    }

    // Any dust is returned to the party posting the NFT
    Ok(remaining_amount.checked_add(remaining_fee).ok_or(DexloanError::NumericalOverflow)?)
}

pub fn calculate_loan_repayment(
    amount: u64,
    basis_points: u32,
    duration: i64
) -> Result<u64> {
    let annual_fee = calculate_fee_from_basis_points(amount as u128, basis_points as u128)?;
    msg!("annual interest fee {}", annual_fee);
    let fee_divisor = (31_536_000 as f64) / (duration as f64);
    msg!("fee_divisor {}", fee_divisor);
    let pro_rata_fee = (annual_fee as f64 / fee_divisor).round() as u64;
    msg!("pro_rata_fee {}", pro_rata_fee);
    Ok(amount + pro_rata_fee)
}