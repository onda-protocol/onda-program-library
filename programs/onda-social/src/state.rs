use anchor_lang::{prelude::*, solana_program::keccak};
use borsh::{BorshDeserialize, BorshSerialize};
use spl_account_compression::Node;

pub const ENTRY_PREFIX: &str = "entry";
pub const FORUM_CONFIG_SIZE: usize = 8 + 8 + 8 + 1 + 32 + 60; // 60 bytes padding
pub const LIKES_PREFIX: &str = "likes";
pub const LIKES_SIZE: usize = 8 + 8;
pub const LIKE_AMOUNT_LAMPORTS: u64 = 100_000;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum RestrictionType {
    None,
    Collection { address: Pubkey },
    Mint { address: Pubkey },
}

#[account]
pub struct ForumConfig {
    pub total_capacity: u64,
    pub post_count: u64,
    pub restriction: RestrictionType,
}

impl ForumConfig {
    pub fn increment_post_count(&mut self) {
        self.post_count = self.post_count.saturating_add(1);
    }

    pub fn contains_post_capacity(&self, requested_capacity: u64) -> bool {
        let remaining_posts = self.total_capacity.saturating_sub(self.post_count);
        requested_capacity <= remaining_posts
    }
}

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_PROFILE_SIZE: usize = 8 + 4 + MAX_NAME_LENGTH + 1 + 32;
pub const PROFILE_PREFIX: &str = "profile";

#[account]
pub struct Profile {
    pub name: String,
    pub mint: Option<Pubkey>,
}

#[account]
pub struct LikeRecord {
    pub amount: u64,
}

impl LikeRecord {
    pub fn increment_like_count(&mut self) {
        self.amount = self.amount.saturating_add(1);
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum OndaSocialEventType {
    /// Marker for 0 data.
    Uninitialized,
    /// Leaf schema event.
    LeafSchemaEvent,
}


#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct LeafSchemaEvent {
    pub event_type: OndaSocialEventType,
    pub version: Version,
    pub schema: LeafSchema,
    pub leaf_hash: [u8; 32],
}

impl LeafSchemaEvent {
    pub fn new(version: Version, schema: LeafSchema, leaf_hash: [u8; 32]) -> Self {
        Self {
            event_type: OndaSocialEventType::LeafSchemaEvent,
            version,
            schema,
            leaf_hash,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum Version {
    V1,
}

impl Default for Version {
    fn default() -> Self {
        Version::V1
    }
}

impl Version {
    pub fn to_bytes(&self) -> u8 {
        match self {
            Version::V1 => 1,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum DataV1 {
    TextPost { title: String, body: String },
    ImagePost { title: String, src: String },
    LinkPost { title: String, url: String },
    Comment { post: Pubkey, parent: Option<Pubkey>, body: String },
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum LeafSchema {
    V1 {
        id: Pubkey,
        author: Pubkey,
        created_at: i64,
        edited_at: Option<i64>,
        nonce: u64,
        data_hash: [u8; 32],
    },
}

impl Default for LeafSchema {
  fn default() -> Self {
      Self::V1 {
          id: Default::default(),
          author: Default::default(),
          created_at: Default::default(),
          edited_at: None,
          nonce: 0,
          data_hash: [0; 32],
      }
  }
}

impl LeafSchema {
  pub fn new_v0(
      id: Pubkey,
      author: Pubkey,
      created_at: i64,
      edited_at: Option<i64>,
      nonce: u64,
      data_hash: [u8; 32],
  ) -> Self {
    Self::V1 {
        id,
        author,
        created_at,
        edited_at,
        nonce,
        data_hash,
      }
  }

  pub fn version(&self) -> Version {
      match self {
          LeafSchema::V1 { .. } => Version::V1,
      }
  }

  pub fn id(&self) -> Pubkey {
      match self {
          LeafSchema::V1 { id, .. } => *id,
      }
  }

  pub fn nonce(&self) -> u64 {
      match self {
          LeafSchema::V1 { nonce, .. } => *nonce,
      }
  }

  pub fn data_hash(&self) -> [u8; 32] {
      match self {
          LeafSchema::V1 { data_hash, .. } => *data_hash,
      }
  }

  pub fn to_event(&self) -> LeafSchemaEvent {
      msg!("to_event: {:?}", self.clone().id());
      LeafSchemaEvent::new(self.version(), self.clone(), self.to_node())
  }

  pub fn to_node(&self) -> Node {
      let hashed_leaf = match self {
          LeafSchema::V1 {
              id,
              author,
              created_at,
              edited_at,
              nonce,
              data_hash,
          } => keccak::hashv(&[
              &[self.version().to_bytes()],
              id.as_ref(),
              author.as_ref(),
              created_at.to_le_bytes().as_ref(),
              edited_at.unwrap_or(0).to_le_bytes().as_ref(),
              nonce.to_le_bytes().as_ref(),
              data_hash.as_ref(),
          ])
          .to_bytes(),
      };
      hashed_leaf
  }
}
