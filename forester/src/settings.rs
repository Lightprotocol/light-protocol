use std::fmt;
use std::fmt::{Display, Formatter};

pub enum SettingsKey {
    Payer,
    StateMerkleTreePubkey,
    NullifierQueuePubkey,
    AddressMerkleTreePubkey,
    AddressMerkleTreeQueuePubkey,
    RegistryPubkey,
}

impl Display for SettingsKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SettingsKey::Payer => "PAYER",
                SettingsKey::StateMerkleTreePubkey => "STATE_MERKLE_TREE_PUBKEY",
                SettingsKey::NullifierQueuePubkey => "NULLIFIER_QUEUE_PUBKEY",
                SettingsKey::RegistryPubkey => "REGISTRY_PUBKEY",
                SettingsKey::AddressMerkleTreePubkey => "ADDRESS_MERKLE_TREE_PUBKEY",
                SettingsKey::AddressMerkleTreeQueuePubkey => "ADDRESS_MERKLE_TREE_QUEUE_PUBKEY",
            }
        )
    }
}
