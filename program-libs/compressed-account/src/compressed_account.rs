use std::collections::HashMap;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_hasher::{Hasher, Poseidon};
use solana_program::pubkey::Pubkey;

use crate::{
    address::pack_account,
    hash_to_bn254_field_size_be,
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext, zero_copy::ZCompressedAccount,
    },
    CompressedAccountError,
};

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}
impl CompressedAccountWithMerkleContext {
    pub fn hash(&self) -> Result<[u8; 32], CompressedAccountError> {
        self.compressed_account.hash::<Poseidon>(
            &self.merkle_context.merkle_tree_pubkey,
            &self.merkle_context.leaf_index,
        )
    }
}

impl CompressedAccountWithMerkleContext {
    pub fn into_read_only(
        &self,
        root_index: Option<u16>,
    ) -> Result<ReadOnlyCompressedAccount, CompressedAccountError> {
        let account_hash = self.hash()?;
        let merkle_context = if root_index.is_none() {
            let mut merkle_context = self.merkle_context;
            merkle_context.prove_by_index = false;
            merkle_context
        } else {
            self.merkle_context
        };
        Ok(ReadOnlyCompressedAccount {
            account_hash,
            merkle_context,
            root_index: root_index.unwrap_or_default(),
        })
    }

    pub fn pack(
        &self,
        root_index: Option<u16>,
        remaining_accounts: &mut HashMap<Pubkey, usize>,
    ) -> Result<PackedCompressedAccountWithMerkleContext, CompressedAccountError> {
        Ok(PackedCompressedAccountWithMerkleContext {
            compressed_account: self.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: pack_account(
                    &self.merkle_context.merkle_tree_pubkey,
                    remaining_accounts,
                ),
                nullifier_queue_pubkey_index: pack_account(
                    &self.merkle_context.nullifier_queue_pubkey,
                    remaining_accounts,
                ),
                leaf_index: self.merkle_context.leaf_index,
                prove_by_index: root_index.is_none(),
            },
            root_index: root_index.unwrap_or_default(),
            read_only: false,
        })
    }
}
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: MerkleContext,
    pub root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    /// Index of leaf in queue. Placeholder of batched Merkle tree updates
    /// currently unimplemented.
    pub prove_by_index: bool,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

pub fn pack_compressed_accounts(
    compressed_accounts: &[CompressedAccountWithMerkleContext],
    root_indices: &[Option<u16>],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedCompressedAccountWithMerkleContext> {
    compressed_accounts
        .iter()
        .zip(root_indices.iter())
        .map(|(x, root_index)| {
            let mut merkle_context = x.merkle_context;
            let root_index = if let Some(root) = root_index {
                *root
            } else {
                merkle_context.prove_by_index = false;
                0
            };

            PackedCompressedAccountWithMerkleContext {
                compressed_account: x.compressed_account.clone(),
                merkle_context: pack_merkle_context(&[merkle_context], remaining_accounts)[0],
                root_index,
                read_only: false,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_output_compressed_accounts(
    compressed_accounts: &[CompressedAccount],
    merkle_trees: &[Pubkey],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<OutputCompressedAccountWithPackedContext> {
    compressed_accounts
        .iter()
        .zip(merkle_trees.iter())
        .map(|(x, tree)| OutputCompressedAccountWithPackedContext {
            compressed_account: x.clone(),
            merkle_tree_index: pack_account(tree, remaining_accounts),
        })
        .collect::<Vec<_>>()
}
pub fn pack_merkle_context(
    merkle_context: &[MerkleContext],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedMerkleContext> {
    merkle_context
        .iter()
        .map(|merkle_context| PackedMerkleContext {
            leaf_index: merkle_context.leaf_index,
            merkle_tree_pubkey_index: pack_account(
                &merkle_context.merkle_tree_pubkey,
                remaining_accounts,
            ),
            nullifier_queue_pubkey_index: pack_account(
                &merkle_context.nullifier_queue_pubkey,
                remaining_accounts,
            ),
            prove_by_index: merkle_context.prove_by_index,
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccount {
    pub owner: Pubkey,
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl CompressedAccount {
    pub fn hash_with_hashed_values<H: Hasher>(
        &self,
        &owner_hashed: &[u8; 32],
        &merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let capacity = 3
            + std::cmp::min(self.lamports, 1) as usize
            + self.address.is_some() as usize
            + self.data.is_some() as usize * 2;
        let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
        vec.push(owner_hashed.as_slice());

        // leaf index and merkle tree pubkey are used to make every compressed account hash unique
        let leaf_index = leaf_index.to_le_bytes();
        vec.push(leaf_index.as_slice());

        vec.push(merkle_tree_hashed.as_slice());

        // Lamports are only hashed if non-zero to safe CU
        // For safety we prefix the lamports with 1 in 1 byte.
        // Thus even if the discriminator has the same value as the lamports, the hash will be different.
        let mut lamports_bytes = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        if self.lamports != 0 {
            lamports_bytes[1..].copy_from_slice(&self.lamports.to_le_bytes());
            vec.push(lamports_bytes.as_slice());
        }

        if self.address.is_some() {
            vec.push(self.address.as_ref().unwrap().as_slice());
        }

        let mut discriminator_bytes = [2, 0, 0, 0, 0, 0, 0, 0, 0];
        if let Some(data) = &self.data {
            discriminator_bytes[1..].copy_from_slice(&data.discriminator);
            vec.push(&discriminator_bytes);
            vec.push(&data.data_hash);
        }
        let hash = H::hashv(&vec)?;
        Ok(hash)
    }

    pub fn hash<H: Hasher>(
        &self,
        &merkle_tree_pubkey: &Pubkey,
        leaf_index: &u32,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let hashed_mt = hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
            .unwrap()
            .0;
        self.hash_with_hashed_values::<H>(
            &hash_to_bn254_field_size_be(&self.owner.to_bytes())
                .unwrap()
                .0,
            &hashed_mt,
            leaf_index,
        )
    }
}

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl ZCompressedAccount<'_> {
    pub fn hash_with_hashed_values<H: Hasher>(
        &self,
        &owner_hashed: &[u8; 32],
        &merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let capacity = 3
            + std::cmp::min(u64::from(self.lamports), 1) as usize
            + self.address.is_some() as usize
            + self.data.is_some() as usize * 2;
        let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
        vec.push(owner_hashed.as_slice());

        // leaf index and merkle tree pubkey are used to make every compressed account hash unique
        let leaf_index = leaf_index.to_le_bytes();
        vec.push(leaf_index.as_slice());

        vec.push(merkle_tree_hashed.as_slice());

        // Lamports are only hashed if non-zero to safe CU
        // For safety we prefix the lamports with 1 in 1 byte.
        // Thus even if the discriminator has the same value as the lamports, the hash will be different.
        let mut lamports_bytes = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        if self.lamports != 0 {
            lamports_bytes[1..].copy_from_slice(&(u64::from(self.lamports)).to_le_bytes());
            vec.push(lamports_bytes.as_slice());
        }

        if self.address.is_some() {
            vec.push(self.address.as_ref().unwrap().as_slice());
        }

        let mut discriminator_bytes = [2, 0, 0, 0, 0, 0, 0, 0, 0];
        if let Some(data) = &self.data {
            discriminator_bytes[1..].copy_from_slice(data.discriminator.as_slice());
            vec.push(&discriminator_bytes);
            vec.push(data.data_hash.as_slice());
        }
        let hash = H::hashv(&vec)?;
        Ok(hash)
    }

    pub fn hash<H: Hasher>(
        &self,
        &merkle_tree_pubkey: &Pubkey,
        leaf_index: &u32,
    ) -> Result<[u8; 32], CompressedAccountError> {
        self.hash_with_hashed_values::<H>(
            &hash_to_bn254_field_size_be(&self.owner.to_bytes())
                .unwrap()
                .0,
            &hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0,
            leaf_index,
        )
    }
}

#[cfg(test)]
mod tests {
    use light_hasher::Poseidon;

    use super::*;
    /// Tests:
    /// 1. functional with all inputs set
    /// 2. no data
    /// 3. no address
    /// 4. no address and no lamports
    /// 5. no address and no data
    /// 6. no address, no data, no lamports
    #[test]
    fn test_compressed_account_hash() {
        let owner = Pubkey::new_unique();
        let address = [1u8; 32];
        let data = CompressedAccountData {
            discriminator: [1u8; 8],
            data: vec![2u8; 32],
            data_hash: [3u8; 32],
        };
        let lamports = 100;
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: Some(data.clone()),
        };
        let merkle_tree_pubkey = Pubkey::new_unique();
        let leaf_index = 1;
        let hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(hash, hash_manual);
        assert_eq!(hash.len(), 32);

        // no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: None,
        };
        let no_data_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();

        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
        ])
        .unwrap();
        assert_eq!(no_data_hash, hash_manual);
        assert_ne!(hash, no_data_hash);

        // no address
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_hash, hash_manual);
        assert_ne!(hash, no_address_hash);
        assert_ne!(no_data_hash, no_address_hash);

        // no address no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_no_lamports_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_no_lamports_hash, hash_manual);
        assert_ne!(hash, no_address_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_lamports_hash);

        // no address and no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: None,
        };
        let no_address_no_data_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_hash, hash_manual);
        assert_ne!(hash, no_address_no_data_hash);
        assert_ne!(no_data_hash, no_address_no_data_hash);
        assert_ne!(no_address_hash, no_address_no_data_hash);
        assert_ne!(no_address_no_lamports_hash, no_address_no_data_hash);

        // no address, no data, no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: None,
        };
        let no_address_no_data_no_lamports_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_no_lamports_hash, hash_manual);
        assert_ne!(no_address_no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(
            no_address_no_lamports_hash,
            no_address_no_data_no_lamports_hash
        );
    }

    /// Tests:
    /// 1. functional with all inputs set
    /// 2. no data
    /// 3. no address
    /// 4. no address and no lamports
    /// 5. no address and no data
    /// 6. no address, no data, no lamports
    #[test]
    fn test_zcompressed_account_hash() {
        let owner = Pubkey::new_unique();
        let address = [1u8; 32];
        let data = CompressedAccountData {
            discriminator: [1u8; 8],
            data: vec![2u8; 32],
            data_hash: [3u8; 32],
        };
        let lamports = 100;
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: Some(data.clone()),
        };
        let merkle_tree_pubkey = Pubkey::new_unique();
        let leaf_index = 1;
        let hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(hash, hash_manual);
        assert_eq!(hash.len(), 32);

        // no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: None,
        };
        let no_data_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();

        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
        ])
        .unwrap();
        assert_eq!(no_data_hash, hash_manual);
        assert_ne!(hash, no_data_hash);

        // no address
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_hash, hash_manual);
        assert_ne!(hash, no_address_hash);
        assert_ne!(no_data_hash, no_address_hash);

        // no address no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_no_lamports_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_no_lamports_hash, hash_manual);
        assert_ne!(hash, no_address_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_lamports_hash);

        // no address and no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: None,
        };
        let no_address_no_data_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_hash, hash_manual);
        assert_ne!(hash, no_address_no_data_hash);
        assert_ne!(no_data_hash, no_address_no_data_hash);
        assert_ne!(no_address_hash, no_address_no_data_hash);
        assert_ne!(no_address_no_lamports_hash, no_address_no_data_hash);

        // no address, no data, no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: None,
        };
        let no_address_no_data_no_lamports_hash = compressed_account
            .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes())
                .unwrap()
                .0
                .as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0
                .as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_no_lamports_hash, hash_manual);
        assert_ne!(no_address_no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(
            no_address_no_lamports_hash,
            no_address_no_data_no_lamports_hash
        );
    }
}
