use {
    std::{slice::Iter},
    anchor_lang::{
        prelude::*,
        system_program,
        solana_program::{
            program::{invoke, invoke_signed},
        },
    },
    anchor_spl::token::{TokenAccount},
    mpl_token_metadata::{
        instruction::{freeze_delegated_account, thaw_delegated_account},
        state::{Metadata}
    },
};
use crate::constants::*;
use crate::state::{Rental, Collection, TokenManager};
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
    issuer: AccountInfo<'info>,
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
    let issuer_pubkey = issuer.key();
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

pub fn maybe_delegate_and_freeze_token_account<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    token_account: &mut Account<'info, TokenAccount>,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    edition: AccountInfo<'info>,
    issuer: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
) -> Result<()> {
    if token_account.delegate.is_some() {
        if !token_account.is_frozen() && token_account.delegate.unwrap() != token_manager.key()  {
            anchor_spl::token::revoke(
                CpiContext::new(
                    token_program.clone(),
                    anchor_spl::token::Revoke {
                        source: token_account.to_account_info(),
                        authority: authority.clone(),
                    }
                )
            )?;

            delegate_and_freeze_token_account(
                token_manager,
                token_program,
                token_account.to_account_info(),
                authority,
                mint,
                edition,
                issuer,
            )?;
        } else if token_account.delegate.unwrap() != token_manager.key() || token_account.delegated_amount != 1 {
            return err!(DexloanError::InvalidDelegate);
        }
    } else {
        delegate_and_freeze_token_account(
            token_manager,
            token_program,
            token_account.to_account_info(),
            authority,
            mint,
            edition,
            issuer,
        )?;
    }

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

    if from_token_account.key() != to_token_account.key() {
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
    }

    Ok(())
}

pub fn calculate_widthdawl_amount<'info>(rental: &mut Account<'info, Rental>, unix_timestamp: i64) -> Result<u64> {
    require!(rental.current_start.is_some(), DexloanError::InvalidState);
    require!(rental.current_expiry.is_some(), DexloanError::InvalidState);

    let start = rental.current_start.unwrap() as f64;
    let end = rental.current_expiry.unwrap() as f64;
    let now = unix_timestamp as f64;
    let balance = rental.escrow_balance as f64;

    if now > end {
        return Ok(rental.escrow_balance)
    }

    let fraction = (now - start) / (end - start);
    let withdrawl_amount = balance * fraction;

    Ok(withdrawl_amount.round() as u64)
}

pub fn transfer_from_escrow(
    escrow: &mut AccountInfo,
    to: &mut AccountInfo,
    amount: u64,
) -> Result<()> {
    **escrow.try_borrow_mut_lamports()? = escrow
        .lamports()
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidArgument)?;
    **to.try_borrow_mut_lamports()? = to
        .lamports()
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;
    
    Ok(())
}

// TODO pay creator fees on escrow withdrawls!
pub fn withdraw_from_rental_escrow<'info>(
    rental: & mut Account<'info, Rental>,
    rental_escrow: & mut AccountInfo<'info>,
    lender: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    metadata_info: &AccountInfo<'info>,
    remaining_accounts: & mut Iter<AccountInfo<'info>>,
    unix_timestamp: i64,
) -> Result<u64> {
    require_keys_eq!(lender.key(), rental.lender);

    let amount = calculate_widthdawl_amount(rental, unix_timestamp)?;
    msg!("Withdrawing {} lamports to lender from escrow balance ", amount);

    let remaining_amount = pay_creator_fees(
        amount,
        rental.creator_basis_points,
        mint,
        metadata_info,
        rental_escrow,
        remaining_accounts
    )?;

    let mint_pubkey = mint.key();
    let lender_pubkey = lender.key();
    let signer_bump = &[rental.escrow_bump];
    let signer_seeds = &[&[
        Rental::ESCROW_PREFIX,
        mint_pubkey.as_ref(),
        lender_pubkey.as_ref(),
        signer_bump
    ][..]];
    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::system_instruction::transfer(
            &rental_escrow.key(),
            &rental.lender,
            remaining_amount,
        ),
        &[
            rental_escrow.to_account_info(),
            lender.to_account_info(),
        ],
        signer_seeds
    )?;

    let remaining_amount = rental.escrow_balance - amount;
    rental.escrow_balance = remaining_amount;
    rental.current_start = Some(unix_timestamp);

    Ok(remaining_amount)
}

// If a call option is exercised or a loan repossessed while a rental is active
// Then any unearned balance must be paid back to the rental's borrower
pub fn settle_rental_escrow_balance<'info>(
    rental: & mut Account<'info, Rental>,
    rental_escrow: & mut AccountInfo<'info>,
    lender: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    metadata_info: &AccountInfo<'info>,
    remaining_accounts: & mut Iter<AccountInfo<'info>>,
    unix_timestamp: i64,
) -> Result<()> {
    let remaining_escrow_balance = withdraw_from_rental_escrow(
        rental,
        rental_escrow,
        lender,
        mint,
        metadata_info,
        remaining_accounts,
        unix_timestamp,
    )?;

    if rental.borrower.is_some() {
        let borrower = next_account_info(remaining_accounts)?;
        require_keys_eq!(borrower.key(), rental.borrower.unwrap());

        msg!("Returning {} lamports to borrower {} from escrow balance", remaining_escrow_balance, borrower.key());        

        let mint_pubkey = mint.key();
        let lender_pubkey = lender.key();
        let signer_bump = &[rental.escrow_bump];
        let signer_seeds = &[&[
            Rental::ESCROW_PREFIX,
            mint_pubkey.as_ref(),
            lender_pubkey.as_ref(),
            signer_bump
        ][..]];
        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::system_instruction::transfer(
                &rental_escrow.key(),
                &rental.borrower.unwrap(),
                remaining_escrow_balance,
            ),
            &[
                rental_escrow.to_account_info(),
                borrower.to_account_info(),
            ],
            signer_seeds
        )?;

        
    }

    rental.escrow_balance = 0;

    Ok(())
}



pub fn process_payment_to_rental_escrow<'info>(
    rental: &mut Account<'info, Rental>,
    rental_escrow: AccountInfo<'info>,
    borrower: AccountInfo<'info>,
    days: u16,
) -> Result<()> {
    let amount = u64::from(days).checked_mul(rental.amount).ok_or(DexloanError::NumericalOverflow)?;

    msg!("Paying {} lamports to rental escrow", amount);

    rental.escrow_balance = rental.escrow_balance + amount;

    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &rental.borrower.unwrap(),
            &rental_escrow.key(),
            amount,
        ),
        &[
            borrower.to_account_info(),
            rental_escrow.to_account_info(),
        ]
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

pub fn assert_collection_valid<'a>(
    metadata: &AccountInfo<'a>,
    mint: Pubkey,
    collection_pda: Pubkey,
    program_id: Pubkey,
) -> Result<()> {
    let metadata = Metadata::deserialize(
        &mut metadata.data.borrow_mut().as_ref()
    )?;

    require_keys_eq!(metadata.mint, mint.key(), DexloanError::InvalidMint);

    match metadata.collection {
        Some(collection) => {
            let seeds = &[
                Collection::PREFIX,
                collection.key.as_ref(),
            ];
            let (address, _) = Pubkey::find_program_address(
                seeds, 
                &program_id
            );

            require_keys_eq!(address, collection_pda, DexloanError::InvalidCollection);
        }
        None => {
            return err!(DexloanError::InvalidCollection);
        }
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
    amount: u64,
    basis_points: u16,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &mut AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
) -> Result<u64> {
    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    )?;

    if metadata.mint != mint.key() {
        return  err!(DexloanError::InvalidMint);
    }

    assert_metadata_valid(
        &metadata_info,
        &mint
    )?;

    let total_fee = calculate_fee_from_basis_points(amount as u128, basis_points as u128)?;
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
                require_keys_eq!(current_creator_info.key(), creator.address);

                if creator_fee > 0 {
                    if fee_payer.owner.key() == system_program::ID {
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
                    } else {
                        transfer_from_escrow(
                            fee_payer,
                            &mut current_creator_info.to_account_info(),
                            creator_fee
                        )?;
                    }
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

pub fn pay_creator_royalties<'a>(
    amount: u64,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &mut AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
) -> Result<u64> {
    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    )?;
    let basis_points = metadata.data.seller_fee_basis_points;
    let remaining_amount = pay_creator_fees(
        amount,
        basis_points,
        mint,
        metadata_info,
        fee_payer,
        remaining_accounts,
    )?;

    Ok(remaining_amount)
}

pub fn calculate_loan_repayment(
    amount: u64,
    basis_points: u32,
    duration: i64,
    is_overdue: bool,
) -> Result<u64> {
    let annual_fee = calculate_fee_from_basis_points(amount as u128, basis_points as u128)?;

    let interest_due = annual_fee.checked_mul(duration as u64)
        .ok_or(DexloanError::NumericalOverflow)?
        .checked_div(SECONDS_PER_YEAR as u64)
        .ok_or(DexloanError::NumericalOverflow)?;

    let mut amount_due = amount.checked_add(interest_due).ok_or(DexloanError::NumericalOverflow)?;
    msg!("interest_due {}", interest_due);

    if is_overdue {
        let late_repayment_fee = calculate_fee_from_basis_points(amount as u128, LATE_REPAYMENT_FEE_BASIS_POINTS)?;
        msg!("late_repayment_fee {}", late_repayment_fee);
        amount_due = amount_due.checked_add(late_repayment_fee).ok_or(DexloanError::NumericalOverflow)?;
    }
    
    
    Ok(amount_due)
}