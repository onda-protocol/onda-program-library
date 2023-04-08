use anchor_lang::{prelude::*, solana_program::keccak};
use borsh::{BorshDeserialize, BorshSerialize};
use spl_account_compression::Node;

pub const ASSET_PREFIX: &str = "asset";
pub const TREE_AUTHORITY_SIZE: usize = 32 + 32 + 8 + 8 + 15; // 15 bytes padding

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct PostArgs {
  pub body: String,
}

#[account]
#[derive(Copy, Debug, PartialEq, Eq)]
pub struct TreeConfig {
    pub tree_creator: Pubkey,
    pub tree_delegate: Pubkey,
    pub total_post_capacity: u64,
    pub post_count: u64,
}

impl TreeConfig {
    pub fn increment_post_count(&mut self) {
        self.post_count = self.post_count.saturating_add(1);
    }

    pub fn contains_post_capacity(&self, requested_capacity: u64) -> bool {
        let remaining_posts = self.total_post_capacity.saturating_sub(self.post_count);
        requested_capacity <= remaining_posts
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum OndaSocialEventType {
    /// Marker for 0 data.
    Uninitialized,
    /// Leaf schema event.
    LeafSchemaEvent,
}


#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
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

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum LeafSchema {
    V1 {
        id: Pubkey,
        owner: Pubkey,
        delegate: Pubkey,
        nonce: u64,
        data_hash: [u8; 32],
    },
}

impl Default for LeafSchema {
  fn default() -> Self {
      Self::V1 {
          id: Default::default(),
          owner: Default::default(),
          delegate: Default::default(),
          nonce: 0,
          data_hash: [0; 32],
      }
  }
}

impl LeafSchema {
  pub fn new_v0(
      id: Pubkey,
      owner: Pubkey,
      delegate: Pubkey,
      nonce: u64,
      data_hash: [u8; 32],
  ) -> Self {
      Self::V1 {
          id,
          owner,
          delegate,
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
      LeafSchemaEvent::new(self.version(), self.clone(), self.to_node())
  }

  pub fn to_node(&self) -> Node {
      let hashed_leaf = match self {
          LeafSchema::V1 {
              id,
              owner,
              delegate,
              nonce,
              data_hash,
          } => keccak::hashv(&[
              &[self.version().to_bytes()],
              id.as_ref(),
              owner.as_ref(),
              delegate.as_ref(),
              nonce.to_le_bytes().as_ref(),
              data_hash.as_ref(),
          ])
          .to_bytes(),
      };
      hashed_leaf
  }
}
