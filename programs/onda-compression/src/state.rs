use anchor_lang::{prelude::*, solana_program::keccak};
use borsh::{BorshDeserialize, BorshSerialize};
use spl_account_compression::Node;

pub const ENTRY_PREFIX: &str = "entry";
pub const BASE_FORUM_CONFIG_SIZE: usize = 8 + 8 + 8 + 32 + 1;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum RestrictionType {
    Collection { address: Pubkey },
    Mint { address: Pubkey },
}   

#[account]
pub struct ForumConfig {
    pub total_capacity: u64,
    pub post_count: u64,
    pub admin: Pubkey,
    pub gate: Option<Vec<RestrictionType>>,
}

impl ForumConfig {
    pub fn get_size(gate: Option<Vec<RestrictionType>>) -> usize {
        BASE_FORUM_CONFIG_SIZE + gate.map(|g| g.len() * 33).map_or(0, |l| if l == 0 { l } else { l + 4 })
    }

    pub fn increment_post_count(&mut self) {
        self.post_count = self.post_count.saturating_add(1);
    }

    pub fn contains_post_capacity(&self, requested_capacity: u64) -> bool {
        let remaining_posts = self.total_capacity.saturating_sub(self.post_count);
        requested_capacity <= remaining_posts
    }

    pub fn set_admin(&mut self, admin: Pubkey) {
        self.admin = admin;
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum DelegateActionType {
    Delete,
}

#[account]
pub struct DelegateAction {
    pub delegate: Pubkey,
    pub action: DelegateActionType,
    pub expiry: i64,
    pub nonce: u64,
}

impl DelegateAction {
    pub const PREFIX: &'static str = "delegate_action";

    pub fn get_size() -> usize {
        8 + 32 + 1
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
    TextPost { title: String, uri: String, nsfw: bool },
    ImagePost { title: String, uri: String, nsfw: bool },
    LinkPost { title: String, uri: String, nsfw: bool },
    VideoPost { title: String, uri: String, nsfw: bool },
    Comment { post: Pubkey, parent: Option<Pubkey>, uri: String },
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
