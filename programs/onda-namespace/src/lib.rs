use anchor_lang::prelude::*;
use onda_compression::{self, state::ForumConfig};

declare_id!("ona67gSygPUkb34U5sgPZK7AkgXDJJrNoi5nrraEHvE");

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_URI_LENGTH: usize = 200;

#[error_code]
pub enum OndaNamespaceError {
  #[msg("Unauthorized.")]
  Unauthorized,
}

#[account]
pub struct Namespace {
    pub name: String,
    pub uri: String,
    pub merkle_tree: Pubkey,
}

// Ensures that a merkle tree can only be used for one namespace.
#[account]
pub struct TreeMarker {
    pub namespace: Pubkey,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateNamespace<'info> {
    pub admin: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        seeds = ["namespace".as_ref(), name.as_ref()],
        bump,
        payer = payer,
        space = 8 + 4 + MAX_NAME_LENGTH + 4 + MAX_URI_LENGTH + 32,
    )]
    pub namespace: Account<'info, Namespace>,
    #[account(
        init,
        seeds = ["tree_marker".as_bytes(), merkle_tree.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + 32,
    )]
    pub tree_marker: Account<'info, TreeMarker>,
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        seeds::program = onda_compression::id(),
        constraint = forum_config.admin.eq(&admin.key()) @OndaNamespaceError::Unauthorized,
        bump,
    )]
    pub forum_config: Account<'info, ForumConfig>,
    /// CHECK: forum_config seed
    pub merkle_tree: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod onda_namespace {
    use super::*;

    pub fn create_namespace(ctx: Context<CreateNamespace>, name: String, uri: String) -> Result<()> {
        let namespace = &mut ctx.accounts.namespace;
        let tree_marker = &mut ctx.accounts.tree_marker;

        namespace.name = puffed_out_string(&name, MAX_NAME_LENGTH);
        namespace.uri = puffed_out_string(&uri, MAX_URI_LENGTH);
        namespace.merkle_tree = ctx.accounts.merkle_tree.key();
        tree_marker.namespace = namespace.key();

        Ok(())
    }
}

pub fn puffed_out_string(s: &str, size: usize) -> String {
    let mut array_of_zeroes = vec![];
    let puff_amount = size - s.len();
    while array_of_zeroes.len() < puff_amount {
        array_of_zeroes.push(0u8);
    }
    s.to_owned() + std::str::from_utf8(&array_of_zeroes).unwrap()
}
