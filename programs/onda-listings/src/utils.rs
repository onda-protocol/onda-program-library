use {
    std::{slice::Iter},
    anchor_lang::{
        prelude::*,
        solana_program::{
            program::{invoke, invoke_signed},
            system_instruction::{transfer}
        },
    },
    anchor_spl::token::{TokenAccount},
    mpl_token_metadata::{
        instruction::{builders, InstructionBuilder, TransferArgs, DelegateArgs, UnlockArgs, LockArgs, RevokeArgs},
        state::{Metadata, TokenStandard}
    },
};

use crate::constants::*;
use crate::state::{Rental, Collection, TokenManager};
use crate::error::*;

pub fn handle_delegate_and_freeze<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    owner: AccountInfo<'info>,
    token_account: AccountInfo<'info>,
    token_record: Option<AccountInfo<'info>>,
    mint: AccountInfo<'info>,
    metadata_info: AccountInfo<'info>,
    edition: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    authorization_rules_program: AccountInfo<'info>,
    authorization_rules: Option<AccountInfo<'info>>,
) -> Result<()> {
    let token_manager_key = token_manager.key();
    let owner_key = owner.key();
    let token_account_key = token_account.key();
    let mint_key = mint.key();
    let metadata_key = metadata_info.key();
    let edition_key = edition.key();
    let system_program_key = system_program.key(); 
    let sysvar_instructions_key = sysvar_instructions.key();
    let token_program_key = token_program.key();
    let authorization_rules_program_key = authorization_rules_program.key();

    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    )?;

    let mut delegate_builder = builders::DelegateBuilder::new();
    
    delegate_builder.delegate(token_manager_key)
        .metadata(metadata_key)
        .master_edition(edition_key)
        .mint(mint.key())
        .token(token_account_key)
        .authority(owner_key)
        .payer(owner_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key);

    let mut delegate_accounts = vec![
        token_manager.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        mint.to_account_info(),
        token_account.to_account_info(),
        owner.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
    ];

    // Conidtionally add authorization rules account
    if authorization_rules.is_some() {
        delegate_accounts.push(authorization_rules.clone().unwrap().to_account_info());
        delegate_accounts.push(authorization_rules_program.to_account_info());

        delegate_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(authorization_rules.clone().unwrap().key());
    }

    if token_record.is_some() {
        delegate_accounts.push(token_record.clone().unwrap().to_account_info());
        delegate_builder.token_record(token_record.clone().unwrap().key());
    }

    let delegate_ix = delegate_builder.build(
        match metadata.token_standard {
            Some(TokenStandard::ProgrammableNonFungible) => {
                DelegateArgs::LockedTransferV1 {
                    amount: 1,
                    locked_address: token_manager.key(),
                    authorization_data: None,
                }
            }, 
            _ => DelegateArgs::StandardV1 { amount: 1 }
        })
        .unwrap()
        .instruction();

    invoke(
        &delegate_ix,
        &delegate_accounts[..],
    )?;

    let mut lock_builder = builders::LockBuilder::new();

    lock_builder
        .authority(token_manager_key)
        .token_owner(owner_key)
        .token(token_account_key)
        .mint(mint_key)
        .metadata(metadata_key)
        .edition(edition_key)
        .payer(owner_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key);

    let signer_bump = &[token_manager.bump];
    let signer_seeds = &[&[
        TokenManager::PREFIX,
        mint_key.as_ref(),
        signer_bump
    ][..]];

    let mut lock_accounts = vec![
        token_manager.to_account_info(),
        owner.to_account_info(),
        token_account.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
    ];

    // Conidtionally add authorization rules account
    if authorization_rules.is_some() {
        let authorization_rules = authorization_rules.unwrap();
        lock_accounts.push(authorization_rules.to_account_info());
        lock_accounts.push(authorization_rules_program.to_account_info());

        lock_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(authorization_rules.key());
    }

    if token_record.is_some() {
        lock_accounts.push(token_record.clone().unwrap().to_account_info());
        lock_builder.token_record(token_record.clone().unwrap().key());
    }

    let lock_ix = lock_builder.build(LockArgs::V1 { authorization_data: None })
        .unwrap()
        .instruction();

    invoke_signed(
        &lock_ix,
        &lock_accounts[..],
        signer_seeds
    )?;

    Ok(())
}

pub fn handle_thaw_and_revoke<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    owner: AccountInfo<'info>,
    token_account: AccountInfo<'info>,
    token_record: Option<AccountInfo<'info>>,
    mint: AccountInfo<'info>,
    metadata_info: AccountInfo<'info>,
    edition: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    authorization_rules_program: AccountInfo<'info>,
    authorization_rules: Option<AccountInfo<'info>>,
) -> Result<()> {
    let token_manager_key = token_manager.key();
    let owner_key = owner.key();
    let token_account_key = token_account.key();
    let mint_key = mint.key();
    let metadata_key = metadata_info.key();
    let edition_key = edition.key();
    let system_program_key = system_program.key(); 
    let sysvar_instructions_key = sysvar_instructions.key();
    let token_program_key = token_program.key();
    let authorization_rules_program_key = authorization_rules_program.key();

    let mut unlock_builder = builders::UnlockBuilder::new();

    unlock_builder.authority(token_manager_key)
        .token_owner(owner_key)
        .token(token_account_key)
        .mint(mint_key)
        .metadata(metadata_key)
        .edition(edition_key)
        .payer(owner_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key);


    let mut unlock_accounts = vec![
        token_manager.to_account_info(),
        owner.to_account_info(),
        token_account.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
    ];

    // Conidtionally add authorization rules account
    if authorization_rules.is_some() {
        let authorization_rules = authorization_rules.clone().unwrap();
        unlock_accounts.push(authorization_rules.to_account_info());
        unlock_accounts.push(authorization_rules_program.to_account_info());

        unlock_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(authorization_rules.key());
    }

    if token_record.is_some() {
        unlock_accounts.push(token_record.clone().unwrap().to_account_info());
        unlock_builder.token_record(token_record.clone().unwrap().key());
    }

    let signer_bump = &[token_manager.bump];
    let signer_seeds = &[&[
        TokenManager::PREFIX,
        mint_key.as_ref(),
        signer_bump
    ][..]];

    let lock_ix = unlock_builder.build(UnlockArgs::V1 { authorization_data: None })
        .unwrap()
        .instruction();

    invoke_signed(
        &lock_ix,
        &unlock_accounts[..],
        signer_seeds
    )?;

    let mut revoke_builder = builders::RevokeBuilder::new();

    revoke_builder.delegate(token_manager_key)
        .metadata(metadata_key)
        .master_edition(edition_key)
        .mint(mint_key)
        .token(token_account_key)
        .authority(owner_key)
        .payer(owner_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key);

    let mut revoke_accounts = vec![
        token_manager.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        mint.to_account_info(),
        token_account.to_account_info(),
        owner.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
    ];

    // Conidtionally add authorization rules account
    if authorization_rules.is_some() {
        let authorization_rules = authorization_rules.unwrap();
        revoke_accounts.push(authorization_rules.to_account_info());
        revoke_accounts.push(authorization_rules_program.to_account_info());

        revoke_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(authorization_rules.key());
    }

    if token_record.is_some() {
        revoke_accounts.push(token_record.clone().unwrap().to_account_info());
        revoke_builder.token_record(token_record.clone().unwrap().key());
    }

    let revoke_ix = &revoke_builder.build(RevokeArgs::LockedTransferV1)
        .unwrap()
        .instruction();

    invoke(
        revoke_ix,
        &revoke_accounts[..]
    )?;

    Ok(())
}

pub fn handle_thaw_and_transfer<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    owner: AccountInfo<'info>,
    owner_token_account: AccountInfo<'info>,
    owner_token_record: Option<AccountInfo<'info>>,
    escrow: AccountInfo<'info>,
    escrow_token_record: Option<AccountInfo<'info>>,
    new_authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    metadata_info: AccountInfo<'info>,
    edition: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    ata_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    authorization_rules_program: AccountInfo<'info>,
    authorization_rules: Option<AccountInfo<'info>>,
) -> Result<()> {
    let token_manager_key = token_manager.key();
    let owner_key = owner.key();
    let owner_token_account_key = owner_token_account.key();
    let escrow_key = escrow.key();
    let new_authority_key = new_authority.key();
    let mint_key = mint.key();
    let metadata_key = metadata_info.key();
    let edition_key = edition.key();
    let system_program_key = system_program.key(); 
    let sysvar_instructions_key = sysvar_instructions.key();
    let token_program_key = token_program.key();
    let ata_program_key = ata_program.key();
    let authorization_rules_program_key = authorization_rules_program.key();

    let signer_bump = &[token_manager.bump];
    let signers_seeds = &[&[
        TokenManager::PREFIX,
        mint_key.as_ref(),
        signer_bump
    ][..]];

    let mut unlock_builder = builders::UnlockBuilder::new();

    unlock_builder.authority(token_manager_key)
        .token_owner(owner_key)
        .token(owner_token_account_key)
        .mint(mint_key)
        .metadata(metadata_key)
        .edition(edition_key)
        .payer(new_authority_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key);


    let mut unlock_accounts = vec![
        token_manager.to_account_info(),
        owner.to_account_info(),
        owner_token_account.to_account_info(),
        new_authority.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
    ];

    // Conidtionally add authorization rules account
    if authorization_rules.is_some() {
        let authorization_rules = authorization_rules.clone().unwrap();
        unlock_accounts.push(authorization_rules.to_account_info());
        unlock_accounts.push(authorization_rules_program.to_account_info());

        unlock_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(authorization_rules.key());
    }

    if owner_token_record.is_some() {
        unlock_accounts.push(owner_token_record.clone().unwrap().to_account_info());
        unlock_builder.token_record(owner_token_record.clone().unwrap().key());
    }

    let lock_ix = unlock_builder.build(UnlockArgs::V1 { authorization_data: None })
        .unwrap()
        .instruction();

    invoke_signed(
        &lock_ix,
        &unlock_accounts[..],
        signers_seeds
    )?;

    let mut transfer_builder = builders::TransferBuilder::new();

    transfer_builder
        .token(owner_token_account_key)
        .token_owner(owner_key)
        .destination(escrow_key)
        .destination_owner(token_manager_key)
        .mint(mint_key)
        .metadata(metadata_key)
        .edition(edition_key)
        .authority(token_manager_key)
        .payer(new_authority_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key)
        .spl_ata_program(ata_program_key);

    let mut transfer_accounts = vec![
        token_manager.to_account_info(),
        owner.to_account_info(),
        owner_token_account.to_account_info(),
        escrow.to_account_info(),
        new_authority.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
        ata_program.to_account_info(),
    ];

    if owner_token_record.is_some() {
        let account = owner_token_record.unwrap();
        transfer_builder.owner_token_record(account.key());
        transfer_accounts.push(account.to_account_info());
    }

    if escrow_token_record.is_some() {
        let account = escrow_token_record.unwrap();
        transfer_builder.destination_token_record(account.key());
        transfer_accounts.push(account.to_account_info());
    }

    if authorization_rules.is_some() {
        let account = authorization_rules.unwrap();
        transfer_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(account.key());
        transfer_accounts.push(authorization_rules_program.to_account_info());
        transfer_accounts.push(account.to_account_info());
    }

    let transfer_ix = transfer_builder.build(TransferArgs::V1 { amount: 1, authorization_data: None })
        .unwrap()
        .instruction();

    invoke_signed(
        &transfer_ix,
        &transfer_accounts[..],
        signers_seeds
    )?;

    // Give control of the token manager escrow to the new owner
    token_manager.authority = Some(new_authority_key);

    Ok(())
}

pub fn claim_from_escrow<'info>(
    token_manager: &mut Account<'info, TokenManager>,
    escrow: AccountInfo<'info>,
    escrow_token_record: Option<AccountInfo<'info>>,
    destination: AccountInfo<'info>,
    destination_owner: AccountInfo<'info>,
    destination_token_record: Option<AccountInfo<'info>>,
    mint: AccountInfo<'info>,
    metadata_info: AccountInfo<'info>,
    edition: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    ata_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    sysvar_instructions: AccountInfo<'info>,
    authorization_rules_program: AccountInfo<'info>,
    authorization_rules: Option<AccountInfo<'info>>,
) -> Result<()> {
    let token_manager_key = token_manager.key();
    let escrow_key = escrow.key();
    let destination_key = destination.key();
    let destination_owner_key = destination_owner.key();
    let mint_key = mint.key();
    let metadata_key = metadata_info.key();
    let edition_key = edition.key();
    let system_program_key = system_program.key(); 
    let sysvar_instructions_key = sysvar_instructions.key();
    let token_program_key = token_program.key();
    let ata_program_key = ata_program.key();
    let authorization_rules_program_key = authorization_rules_program.key();

    let mut transfer_builder = builders::TransferBuilder::new();

    transfer_builder
        .token(escrow_key)
        .token_owner(token_manager_key)
        .destination(destination_key)
        .destination_owner(destination_owner_key)
        .mint(mint_key)
        .metadata(metadata_key)
        .edition(edition_key)
        .authority(token_manager_key)
        .payer(destination_owner_key)
        .system_program(system_program_key)
        .sysvar_instructions(sysvar_instructions_key)
        .spl_token_program(token_program_key)
        .spl_ata_program(ata_program_key);

    let mut transfer_accounts = vec![
        escrow.to_account_info(),
        token_manager.to_account_info(),
        destination.to_account_info(),
        destination_owner.to_account_info(),
        mint.to_account_info(),
        metadata_info.to_account_info(),
        edition.to_account_info(),
        system_program.to_account_info(),
        sysvar_instructions.to_account_info(),
        token_program.to_account_info(),
        ata_program.to_account_info(),
    ];

    if escrow_token_record.is_some() {
        let account = escrow_token_record.unwrap();
        transfer_builder.owner_token_record(account.key());
        transfer_accounts.push(account.to_account_info());
    }

    if destination_token_record.is_some() {
        let account = destination_token_record.unwrap();
        transfer_builder.destination_token_record(account.key());
        transfer_accounts.push(account.to_account_info());
    }

    if authorization_rules.is_some() {
        let account = authorization_rules.unwrap();
        transfer_builder
            .authorization_rules_program(authorization_rules_program_key)
            .authorization_rules(account.key());
        transfer_accounts.push(authorization_rules_program.to_account_info());
        transfer_accounts.push(account.to_account_info());
    }

    let transfer_ix = transfer_builder.build(TransferArgs::V1 { amount: 1, authorization_data: None })
        .unwrap()
        .instruction();

    let signer_bump = &[token_manager.bump];
    let signers_seeds = &[&[
        TokenManager::PREFIX,
        mint_key.as_ref(),
        signer_bump
    ][..]];

    invoke_signed(
        &transfer_ix,
        &transfer_accounts[..],
        signers_seeds
    )?;

    // Close the escrow account
    anchor_spl::token::close_account(
        CpiContext::new_with_signer(
            token_program.clone(),
            anchor_spl::token::CloseAccount {
                account: escrow.clone(),
                destination: destination_owner.clone(),
                authority: token_manager.to_account_info(),
            },
            signers_seeds,
        )
    )?;

    Ok(())
}


pub fn calculate_widthdawl_amount<'info>(rental: &mut Account<'info, Rental>, unix_timestamp: i64) -> Result<u64> {
    require!(rental.current_start.is_some(), ErrorCodes::InvalidState);
    require!(rental.current_expiry.is_some(), ErrorCodes::InvalidState);

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

    let mint_pubkey = mint.key();
    let lender_pubkey = lender.key();
    let signer_bump = &[rental.escrow_bump];
    let signer_seeds = &[&[
        Rental::ESCROW_PREFIX,
        mint_pubkey.as_ref(),
        lender_pubkey.as_ref(),
        signer_bump
    ][..]];

    let amount = calculate_widthdawl_amount(rental, unix_timestamp)?;
    msg!("Withdrawing {} lamports to lender from escrow balance ", amount);

    let remaining_amount = pay_creator_fees_with_signer(
        amount,
        rental.creator_basis_points,
        mint,
        metadata_info,
        rental_escrow,
        remaining_accounts,
        signer_seeds
    )?;

    invoke_signed(
        &transfer(
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
        invoke_signed(
            &transfer(
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
    let amount = u64::from(days).checked_mul(rental.amount).ok_or(ErrorCodes::NumericalOverflow)?;
    let creator_fee = calculate_fee_from_basis_points(amount as u128, rental.creator_basis_points as u128)?;
    let total_amount = amount.checked_add(creator_fee).ok_or(ErrorCodes::NumericalOverflow)?;

    msg!("Paying {} lamports to rental escrow", amount);

    rental.escrow_balance = rental.escrow_balance + amount;

    invoke(
        &transfer(
            &rental.borrower.unwrap(),
            &rental_escrow.key(),
            total_amount,
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
      return err!(ErrorCodes::DerivedKeyInvalid);
    }
  
    if metadata.data_is_empty() {
      return err!(ErrorCodes::MetadataDoesntExist);
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

    require_keys_eq!(metadata.mint, mint.key(), ErrorCodes::InvalidMint);

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

            require_keys_eq!(address, collection_pda, ErrorCodes::InvalidCollection);
            require!(collection.verified, ErrorCodes::InvalidCollection);
        }
        None => {
            return err!(ErrorCodes::InvalidCollection);
        }
    }

    Ok(())
}
  
pub fn calculate_fee_from_basis_points(
    amount: u128,
    basis_points: u128,
) -> Result<u64> {
    let total_fee = basis_points.checked_mul(amount)
        .ok_or(ErrorCodes::NumericalOverflow)?
        .checked_div(10_000)
        .ok_or(ErrorCodes::NumericalOverflow)? as u64;
    
    Ok(total_fee)
}

pub struct CreatorFee<'a> {
    pub amount: u64,
    pub address: Pubkey,
    /// CHECK:
    pub account_info: AccountInfo<'a>
}

pub fn get_creator_fees<'a>(
    amount: u64,
    basis_points: u16,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
) -> Result<(Vec<CreatorFee<'a>>, u64)> {
    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    )?;

    if metadata.mint != mint.key() {
        return  err!(ErrorCodes::InvalidMint);
    }

    assert_metadata_valid(
        &metadata_info,
        &mint
    )?;

    let total_fee = calculate_fee_from_basis_points(amount as u128, basis_points as u128)?;
    let mut remaining_fee = total_fee;
    let remaining_amount = amount
            .checked_sub(total_fee)
            .ok_or(ErrorCodes::NumericalOverflow)?;
    
    let mut fees: Vec<CreatorFee> = Vec::new();

    msg!("Paying {} lamports in royalties", total_fee);
        
    match metadata.data.creators {
        Some(creators) => {
            for creator in creators {
                let pct = creator.share as u128;
                let amount = pct.checked_mul(total_fee as u128)
                        .ok_or(ErrorCodes::NumericalOverflow)?
                        .checked_div(100)
                        .ok_or(ErrorCodes::NumericalOverflow)? as u64;
                remaining_fee = remaining_fee
                        .checked_sub(amount)
                        .ok_or(ErrorCodes::NumericalOverflow)?;

                let current_creator_info = next_account_info(remaining_accounts)?;
                let address = current_creator_info.key();
                require_keys_eq!(address, creator.address);

                fees.push(CreatorFee {
                    amount,
                    address,
                    account_info: current_creator_info.to_account_info()
                });
            }
        }
        None => {
            msg!("No creators found in metadata");
        }
    }

    let remaining = remaining_amount.checked_add(remaining_fee).ok_or(ErrorCodes::NumericalOverflow)?;

    Ok((fees, remaining))
}

pub fn pay_creator_fees<'a>(
    amount: u64,
    basis_points: u16,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &mut AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
) -> Result<u64> {
    let (fees, remaining_amount) = get_creator_fees(
        amount,
        basis_points,
        mint,
        metadata_info,
        remaining_accounts,
    )?;

    for creator_fee in fees {
        invoke(
            &transfer(
                &fee_payer.key(),
                &creator_fee.address,
                creator_fee.amount,
            ),
            &[
                fee_payer.to_account_info(),
                creator_fee.account_info,
            ],
        )?;
    }

    Ok(remaining_amount)
}

pub fn pay_creator_fees_with_signer<'a>(
    amount: u64,
    basis_points: u16,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &mut AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64> {
    let (fees, remaining_amount) = get_creator_fees(
        amount,
        basis_points,
        mint,
        metadata_info,
        remaining_accounts,
    )?;

    for creator_fee in fees {
        invoke_signed(
            &transfer(
                &fee_payer.key(),
                &creator_fee.address,
                creator_fee.amount,
            ),
            &[
                fee_payer.to_account_info(),
                creator_fee.account_info,
            ],
            signer_seeds,
        )?;
    }

    Ok(remaining_amount)
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
    let (fees, remaining_amount) = get_creator_fees(
        amount,
        basis_points,
        mint,
        metadata_info,
        remaining_accounts,
    )?;

    for creator_fee in fees {
        invoke(
            &transfer(
                &fee_payer.key(),
                &creator_fee.address,
                creator_fee.amount,
            ),
            &[
                fee_payer.to_account_info(),
                creator_fee.account_info,
            ],
        )?;
    }

    Ok(remaining_amount)
}

pub fn pay_creator_royalties_with_signer<'a>(
    amount: u64,
    mint: &AccountInfo<'a>,
    metadata_info: &AccountInfo<'a>,
    fee_payer: &mut AccountInfo<'a>,
    remaining_accounts: &mut Iter<AccountInfo<'a>>,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64> {
    let metadata = Metadata::deserialize(
        &mut metadata_info.data.borrow_mut().as_ref()
    )?;
    let basis_points = metadata.data.seller_fee_basis_points;
    let (fees, remaining_amount) = get_creator_fees(
        amount,
        basis_points,
        mint,
        metadata_info,
        remaining_accounts,
    )?;

    for creator_fee in fees {
        invoke_signed(
            &transfer(
                &fee_payer.key(),
                &creator_fee.address,
                creator_fee.amount,
            ),
            &[
                fee_payer.to_account_info(),
                creator_fee.account_info,
            ],
            signer_seeds,
        )?;
    }

    Ok(remaining_amount)
}

pub fn calculate_loan_repayment_fee(
    amount: u64,
    basis_points: u16,
    duration: i64,
    is_overdue: bool,
) -> Result<u64> {
    let annual_fee = calculate_fee_from_basis_points(amount as u128, basis_points as u128)?;

    let mut interest_due = annual_fee.checked_mul(duration as u64)
        .ok_or(ErrorCodes::NumericalOverflow)?
        .checked_div(SECONDS_PER_YEAR as u64)
        .ok_or(ErrorCodes::NumericalOverflow)?;

    // let mut amount_due = amount.checked_add(interest_due).ok_or(ErrorCodes::NumericalOverflow)?;
    msg!("interest_due {}", interest_due);

    if is_overdue {
        let late_repayment_fee = calculate_fee_from_basis_points(amount as u128, LATE_REPAYMENT_FEE_BASIS_POINTS)?;
        msg!("late_repayment_fee {}", late_repayment_fee);
        interest_due = interest_due.checked_add(late_repayment_fee).ok_or(ErrorCodes::NumericalOverflow)?;
    }
    
    
    Ok(interest_due)
}