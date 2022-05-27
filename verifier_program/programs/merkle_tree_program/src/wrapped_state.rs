use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
  program_pack::Pack
};
#[derive(Clone)]
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

impl anchor_lang::AccountSerialize for MerkleTree {
  fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
      // no-op
      Ok(())
  }
}

impl anchor_lang::AccountDeserialize for MerkleTree {
  fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
      MerkleTree::try_deserialize_unchecked(buf)
  }

  fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
      Ok(crate::poseidon_merkle_tree::state::MerkleTree::unpack(buf).map(MerkleTree)?)
  }
}
