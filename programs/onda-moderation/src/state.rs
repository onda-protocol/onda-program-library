use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum Role {
    Owner,
    Admin,
    Moderator,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Member {
    pub address: Pubkey,
    pub role: Role,
}

#[account]
pub struct Team {
    pub forum: Pubkey,
    pub members: Vec<Member>,
}

impl Team {
    pub const PREFIX:&str = "team";

    pub fn get_size(members: usize) -> usize {
        8 + 32 + 4 + members * std::mem::size_of::<Member>()
    }

}