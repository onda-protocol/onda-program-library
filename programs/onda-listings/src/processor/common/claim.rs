use {
  anchor_lang::{prelude::*},
  anchor_spl::{
      associated_token::{AssociatedToken},
      token::{Token, TokenAccount, Mint}
  }
};
use crate::state::{TokenManager};
use crate::error::{ErrorCodes};
use crate::utils::*;
use crate::constants::*;

#[derive(Accounts)]
pub struct Claim<'info> {
  #[account(
      constraint = signer.key() == SIGNER_PUBKEY
  )]
  pub signer: Signer<'info>,
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(
      init_if_needed,
      associated_token::mint = mint,
      associated_token::authority = authority,
      payer = authority,
  )]
  pub destination_token_account: Box<Account<'info, TokenAccount>>,
  #[account(mut)]
  /// CHECK: validated in cpi
  pub destination_token_record: Option<UncheckedAccount<'info>>,
  #[account(
      mut,
      seeds = [
          TokenManager::PREFIX,
          mint.key().as_ref(),
      ],
      bump,
      constraint = token_manager.accounts.loan == false && token_manager.accounts.call_option == false && token_manager.accounts.rental == false @ ErrorCodes::InvalidState,
      constraint = token_manager.authority == Some(authority.key()) @ ErrorCodes::Unauthorized,
  )]   
  pub token_manager: Box<Account<'info, TokenManager>>,
  #[account(
      mut,
      seeds = [
          TokenManager::ESCROW_PREFIX,
          token_manager.key().as_ref(),
      ],
      bump,
      token::mint = mint,
      token::authority = token_manager,
      close = authority,
  )]
  pub escrow_token_account: Box<Account<'info, TokenAccount>>,
  #[account(mut)]
  /// CHECK: contrained on loan_account
  pub escrow_token_record: Option<UncheckedAccount<'info>>,
  /// CHECK: contrained on loan_account
  pub mint: Box<Account<'info, Mint>>,
  #[account(mut)]
  /// CHECK: validated in cpi
  pub metadata: UncheckedAccount<'info>,
  /// CHECK: validated in cpi
  pub edition: UncheckedAccount<'info>,
  /// CHECK: validated in cpi
  pub metadata_program: UncheckedAccount<'info>,
  /// CHECK: validated in cpi
  pub authorization_rules_program: UncheckedAccount<'info>,
  /// CHECK: validated in cpi
  pub authorization_rules: Option<UncheckedAccount<'info>>, 
  pub system_program: Program<'info, System>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  /// CHECK: not supported by anchor? used in cpi
  pub sysvar_instructions: UncheckedAccount<'info>,
  pub rent: Sysvar<'info, Rent>,
}

pub fn handle_claim(ctx: Context<Claim>) -> Result<()> {
  let token_manager = &mut ctx.accounts.token_manager;
  let authority = &mut ctx.accounts.authority;
  let destination_token_account = &mut ctx.accounts.destination_token_account;
  let destination_token_record = &mut ctx.accounts.destination_token_record;
  let escrow_token_account = &mut ctx.accounts.escrow_token_account;
  let escrow_token_record = &mut ctx.accounts.escrow_token_record;
  let mint = &ctx.accounts.mint;
  let edition = &ctx.accounts.edition;
  let metadata_info = &mut ctx.accounts.metadata;
  let authorization_rules_program = &mut ctx.accounts.authorization_rules_program;
  let authorization_rules = &mut ctx.accounts.authorization_rules;
  let token_program = &ctx.accounts.token_program;
  let associated_token_program = &ctx.accounts.associated_token_program;
  let system_program = &ctx.accounts.system_program;
  let sysvar_instructions = &ctx.accounts.sysvar_instructions;

  claim_from_escrow(
      token_manager,
      escrow_token_account.to_account_info(),
      match escrow_token_record {
          Some(account) => Some(account.to_account_info()),
          None => None,
      },
      destination_token_account.to_account_info(),
      authority.to_account_info(),
      match destination_token_record {
          Some(account) => Some(account.to_account_info()),
          None => None,
      },
      mint.to_account_info(),
      metadata_info.to_account_info(),
      edition.to_account_info(),
      token_program.to_account_info(),
      associated_token_program.to_account_info(),
      system_program.to_account_info(),
      sysvar_instructions.to_account_info(),
      authorization_rules_program.to_account_info(),
      match authorization_rules {
          Some(account) => Some(account.to_account_info()),
          None => None,
      },
  )?;

  Ok(())
}
