use anchor_lang::prelude::*;
use onda_compression::{self, program::OndaCompression};

use crate::{state::*, error::*};
pub mod state;
pub mod error;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds = [Team::PREFIX.as_bytes(), merkle_tree.key().as_ref()],
        bump,
        space = Team::get_size(1),
        payer = admin,
    )]
    pub team: Account<'info, Team>,
    /// CHECK: checked in cpi
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub forum_config: UncheckedAccount<'info>,
    pub onda_compression: Program<'info, OndaCompression>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddMember<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: any account
    pub new_member: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [Team::PREFIX.as_bytes(), merkle_tree.key().as_ref()],
        bump,
        realloc = Team::get_size(team.members.len() + 1),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub team: Account<'info, Team>,
    /// CHECK: checked in cpi
    pub merkle_tree: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveMember<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: the account being removed
    pub member: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [Team::PREFIX.as_bytes(), merkle_tree.key().as_ref()],
        bump,
        realloc = Team::get_size(team.members.len() - 1),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub team: Account<'info, Team>,
    /// CHECK: checked in cpi
    pub merkle_tree: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitDeleteAction<'info> {
    #[account(mut)]
    pub member: Signer<'info>,
    #[account(
        mut,
        seeds = [Team::PREFIX.as_bytes(), merkle_tree.key().as_ref()],
        bump,
    )]
    pub team: Account<'info, Team>,
    /// CHECK: checked in cpi
    pub delegate_action: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: checked in cpi
    pub forum_config: UncheckedAccount<'info>,
    pub onda_compression: Program<'info, OndaCompression>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod onda_moderation {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let team = &mut ctx.accounts.team;

        team.forum = *ctx.accounts.forum_config.key;
        team.members.push(Member {
            address: *ctx.accounts.admin.key,
            role: Role::Owner,
        });

        let cpi_program = ctx.accounts.onda_compression.to_account_info();
        let cpi_accounts = onda_compression::cpi::accounts::SetAdmin {
                admin: ctx.accounts.admin.to_account_info(),
                new_admin: ctx.accounts.team.to_account_info(),
                forum_config: ctx.accounts.forum_config.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        onda_compression::cpi::set_admin(cpi_ctx)?;

        Ok(())
    }

    pub fn add_member(ctx: Context<AddMember>, role: Role) -> Result<()> {
        let team = &mut ctx.accounts.team;
        let admin = &ctx.accounts.admin;
        let new_member = &ctx.accounts.new_member;

        // Only allow one owner
        if role == Role::Owner {
            return err!(ErrorCodes::Unauthorized);
        }

        if team.members.iter().any(|m| m.address.eq(&new_member.key())) {
            return err!(ErrorCodes::MemberAlreadyExists);
        }

        let admin_role = team.members.iter().find(|m| m.address.eq(&admin.key())).ok_or(ErrorCodes::Unauthorized)?;

        // Only admins or owners can add members 
        match admin_role.role {
            Role::Owner => {},
            Role::Admin => {},
            _ => {
                return err!(ErrorCodes::Unauthorized);
            },
        }

        team.members.push(Member {
            address: new_member.key(),
            role,
        });

        Ok(())
    }

    pub fn remove_member(ctx: Context<RemoveMember>) -> Result<()> {
        let team = &mut ctx.accounts.team;
        let admin = &ctx.accounts.admin;
        let member = &ctx.accounts.member;

        let member_role = team.members.iter().find(|m| m.address.eq(&member.key())).ok_or(ErrorCodes::MemberNotFound)?;
        let admin_role = team.members.iter().find(|m| m.address.eq(&admin.key())).ok_or(ErrorCodes::Unauthorized)?;

        // Owners cannot be removed
        if member_role.role == Role::Owner {
            return err!(ErrorCodes::Unauthorized);
        }

        // Only admins or owners can remove members 
        match admin_role.role {
            Role::Owner => {},
            Role::Admin => {},
            _ => {
                return err!(ErrorCodes::Unauthorized);
            },
        }

        team.members.retain(|m| !m.address.eq(&member.key()));

        Ok(())
    }

    pub fn init_delete_action(ctx: Context<InitDeleteAction>, nonce: u64) -> Result<()> {    
        let team = &mut ctx.accounts.team;
        let member = &ctx.accounts.member;
        let _member_role = team.members.iter().find(|m| m.address.eq(&member.key())).ok_or(ErrorCodes::MemberNotFound)?;
        
        
        let cpi_program = ctx.accounts.onda_compression.to_account_info();
        let cpi_accounts = onda_compression::cpi::accounts::InitDelegate {
                admin: ctx.accounts.team.to_account_info(),
                delegate: ctx.accounts.member.to_account_info(),
                delegate_action: ctx.accounts.delegate_action.to_account_info(),
                forum_config: ctx.accounts.forum_config.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bump = *ctx.bumps.get("team").unwrap();
        let merkle_tree_key = ctx.accounts.merkle_tree.key();
        let seeds = &[
            Team::PREFIX.as_bytes(),
            merkle_tree_key.as_ref(),
            &[bump]
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            cpi_program,
            cpi_accounts,
            signer_seeds
        );
        onda_compression::cpi::init_delegate(cpi_ctx, nonce)?;

        Ok(())
    }
}
