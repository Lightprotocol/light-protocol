use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
  program_pack::Pack
};
use std::ops::Deref;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MerkleTree(crate::poseidon_merkle_tree::state::MerkleTree);
impl MerkleTree {
  /// The length, in bytes, of the packed representation
  pub const LEN: usize = crate::poseidon_merkle_tree::state::MerkleTree::LEN;
}
impl Owner for MerkleTree {
  fn owner() -> Pubkey {
      crate::ID
  }
}

impl Deref for MerkleTree {
  type Target = crate::poseidon_merkle_tree::state::MerkleTree;

  fn deref(&self) -> &Self::Target {
      &self.0
  }
}

impl anchor_lang::AccountSerialize for MerkleTree {}

impl anchor_lang::AccountDeserialize for MerkleTree {
  fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
      MerkleTree::try_deserialize_unchecked(buf)
  }

  fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
      Ok(crate::poseidon_merkle_tree::state::MerkleTree::unpack(buf).map(MerkleTree)?)
  }
}
/*
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MerkleTreeTmpPda(crate::state::MerkleTreeTmpPda);
impl MerkleTreeTmpPda {
  /// The length, in bytes, of the packed representation
  pub const LEN: usize = crate::state::MerkleTreeTmpPda::LEN;
}
impl Owner for MerkleTreeTmpPda {
  fn owner() -> Pubkey {
      crate::ID
  }
}

impl Deref for MerkleTreeTmpPda {
  type Target = crate::state::MerkleTreeTmpPda;

  fn deref(&self) -> &Self::Target {
      &self.0
  }
}

impl anchor_lang::AccountSerialize for MerkleTreeTmpPda {}

impl anchor_lang::AccountDeserialize for MerkleTreeTmpPda {
  fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
      MerkleTreeTmpPda::try_deserialize_unchecked(buf)
  }

  fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
      Ok(crate::state::MerkleTreeTmpPda::unpack(buf).map(MerkleTreeTmpPda)?)
  }
}
*/
