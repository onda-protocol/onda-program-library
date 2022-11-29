use anchor_lang::{prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{Rental, RentalState, Collection, TokenManager};
use crate::error::{DexloanError};
use crate::utils::*;
use crate::constants::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RentalArgs {
    amount: u64,
    expiry: i64,
    borrower: Option<Pubkey>,
}

#[derive(Accounts)]
pub struct InitRental<'info> {
    #[account(
        constraint = signer.key() == SIGNER_PUBKEY
    )]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = lender,
    )]
    pub deposit_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = lender,
        seeds = [
            Rental::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref(),
        ],
        space = Rental::space(),
        bump,
    )]
    pub rental: Box<Account<'info, Rental>>,    
    #[account(
        init_if_needed,
        payer = lender,
        seeds = [
            TokenManager::PREFIX,
            mint.key().as_ref(),
            lender.key().as_ref()
        ],
        space = TokenManager::space(),
        bump,
    )]   
    pub token_manager: Box<Account<'info, TokenManager>>,
    #[account(
        seeds = [
            Collection::PREFIX,
            collection.mint.as_ref(),
        ],
        bump,
        constraint = collection.config.rental_enabled == true
    )]
    pub collection: Box<Account<'info, Collection>>,
    #[account(constraint = mint.supply == 1)]
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: deserialized and checked
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub edition: UncheckedAccount<'info>,
    /// CHECK: validated in cpi
    pub metadata_program: UncheckedAccount<'info>, 
    /// Misc
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_rental(
  ctx: Context<InitRental>,
  args: RentalArgs,
) -> Result<()> {
    let rental = &mut ctx.accounts.rental;
    let token_manager = &mut ctx.accounts.token_manager;
    let deposit_token_account = &mut ctx.accounts.deposit_token_account;
    let unix_timestamp = ctx.accounts.clock.unix_timestamp;

    assert_collection_valid(
        &ctx.accounts.metadata,
        ctx.accounts.mint.key(),
        ctx.accounts.collection.key(),
        ctx.program_id.clone(),
    )?;

    if unix_timestamp > args.expiry {
        return err!(DexloanError::InvalidExpiry)
    }

    if args.amount == 0 && args.borrower.is_none() {
        return err!(DexloanError::BorrowerNotSpecified)
    }

    // Init
    rental.lender = ctx.accounts.lender.key();
    rental.mint = ctx.accounts.mint.key();
    rental.bump = *ctx.bumps.get("rental").unwrap();
    //
    rental.amount = args.amount;
    rental.escrow_balance = 0;
    rental.expiry = args.expiry;
    rental.state = RentalState::Listed;
    if args.borrower.is_some() {
        rental.borrower = args.borrower;
    }
    //
    token_manager.accounts.rental = true;
    token_manager.bump = *ctx.bumps.get("token_manager").unwrap();

    if deposit_token_account.delegate.is_some() {
        if !deposit_token_account.is_frozen() && deposit_token_account.delegate.unwrap() != token_manager.key()  {
            anchor_spl::token::revoke(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Revoke {
                        source: deposit_token_account.to_account_info(),
                        authority: ctx.accounts.lender.to_account_info(),
                    }
                )
            )?;

            delegate_and_freeze_token_account(
                token_manager,
                ctx.accounts.token_program.to_account_info(),
                deposit_token_account.to_account_info(),
                ctx.accounts.lender.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.edition.to_account_info(),
                ctx.accounts.lender.to_account_info(),
            )?;
        } else if deposit_token_account.delegate.unwrap() != token_manager.key() {
            return err!(DexloanError::InvalidDelegate);
        }
    } else {
        delegate_and_freeze_token_account(
            token_manager,
            ctx.accounts.token_program.to_account_info(),
            deposit_token_account.to_account_info(),
            ctx.accounts.lender.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.edition.to_account_info(),
            ctx.accounts.lender.to_account_info(),
        )?;
    }

    Ok(())
}