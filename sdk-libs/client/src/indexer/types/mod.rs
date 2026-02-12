mod account;
mod interface;
mod proof;
mod queue;
mod signature;
mod token;
mod tree;

pub use account::CompressedAccount;
pub use interface::{
    AccountInterface, ColdContext, ColdData, InterfaceTreeInfo, SolanaAccountData,
};
pub use proof::{
    AccountProofInputs, AddressProofInputs, AddressWithTree, MerkleProof, MerkleProofWithContext,
    NewAddressProofWithContext, RootIndex, ValidityProofWithContext,
};
pub use queue::{
    AddressQueueData, InputQueueData, OutputQueueData, QueueElementsResult, StateQueueData,
};
pub use signature::SignatureWithMetadata;
pub use token::{CompressedTokenAccount, OwnerBalance, TokenBalance};
pub use tree::{AddressMerkleTreeAccounts, NextTreeInfo, StateMerkleTreeAccounts, TreeInfo};

pub struct ProofOfLeaf {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

#[derive(Debug, Clone, PartialEq)]
pub struct QueueInfo {
    pub tree: solana_pubkey::Pubkey,
    pub queue: solana_pubkey::Pubkey,
    pub queue_type: u8,
    pub queue_size: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueInfoResult {
    pub queues: Vec<QueueInfo>,
    pub slot: u64,
}
