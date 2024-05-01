use std::collections::HashMap;

use anchor_lang::prelude::*;
use light_hasher::{Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_be;

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

// TODO: use in PackedCompressedAccountWithMerkleContext and rename to CompressedAccountAndMerkleContext
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
}

pub fn pack_merkle_context(
    merkle_context: &[MerkleContext],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedMerkleContext> {
    let mut merkle_context_packed = merkle_context
        .iter()
        .map(|x| PackedMerkleContext {
            leaf_index: x.leaf_index,
            merkle_tree_pubkey_index: 0,     // will be assigned later
            nullifier_queue_pubkey_index: 0, // will be assigned later
        })
        .collect::<Vec<PackedMerkleContext>>();
    let len: usize = remaining_accounts.len();
    for (i, params) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&params.merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.merkle_tree_pubkey, i + len);
            }
        };
        merkle_context_packed[i].merkle_tree_pubkey_index =
            *remaining_accounts.get(&params.merkle_tree_pubkey).unwrap() as u8;
    }

    let len: usize = remaining_accounts.len();
    for (i, params) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&params.nullifier_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.nullifier_queue_pubkey, i + len);
            }
        };
        merkle_context_packed[i].nullifier_queue_pubkey_index = *remaining_accounts
            .get(&params.nullifier_queue_pubkey)
            .unwrap() as u8;
    }
    merkle_context_packed
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

impl CompressedAccount {
    pub fn hash(&self, &merkle_tree_pubkey: &Pubkey, leaf_index: &u32) -> Result<[u8; 32]> {
        let capacity = 4 + self.address.is_some() as usize + self.data.is_some() as usize * 2;
        let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
        let truncated_owner = hash_to_bn254_field_size_be(&self.owner.to_bytes())
            .unwrap()
            .0;
        vec.push(truncated_owner.as_slice());

        // leaf index and merkle tree pubkey are used to make every compressed account hash unique
        let leaf_index = leaf_index.to_le_bytes();
        vec.push(leaf_index.as_slice());
        let truncated_merkle_tree_pubkey =
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0;
        vec.push(truncated_merkle_tree_pubkey.as_slice());
        let lamports = self.lamports.to_le_bytes();
        vec.push(lamports.as_slice());

        if self.address.is_some() {
            vec.push(self.address.as_ref().unwrap().as_slice());
        }

        if let Some(data) = &self.data {
            // TODO: double check that it is impossible to create a hash collisions for different sized poseidon hash inputs
            // Otherwise we could use padding to prevent a theoretical attack producing a hash collision
            // if self.address.is_none() {
            //     vec.push(&[0u8; 32]);
            // }
            vec.push(&data.discriminator);
            vec.push(&data.data_hash);
        }

        let hash = Poseidon::hashv(&vec).map_err(ProgramError::from)?;
        Ok(hash)
    }
}

pub fn derive_address(merkle_tree_pubkey: &Pubkey, seed: &[u8; 32]) -> Result<[u8; 32]> {
    let hash = match hash_to_bn254_field_size_be(
        [merkle_tree_pubkey.to_bytes(), *seed].concat().as_slice(),
    ) {
        Some(hash) => Ok::<[u8; 32], Error>(hash.0),
        None => return Err(crate::ErrorCode::DeriveAddressError.into()),
    }?;

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

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
            .hash(&merkle_tree_pubkey, &leaf_index)
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
            lamports.to_le_bytes().as_slice(),
            address.as_slice(),
            &data.discriminator,
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
            .hash(&merkle_tree_pubkey, &leaf_index)
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
            lamports.to_le_bytes().as_slice(),
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
            .hash(&merkle_tree_pubkey, &leaf_index)
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
            lamports.to_le_bytes().as_slice(),
            &data.discriminator,
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_hash, hash_manual);
        assert_ne!(hash, no_address_hash);
        assert_ne!(no_data_hash, no_address_hash);

        // no address and no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: None,
        };
        let no_address_no_data_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index)
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
            lamports.to_le_bytes().as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_hash, hash_manual);
        assert_ne!(hash, no_address_no_data_hash);
        assert_ne!(no_data_hash, no_address_no_data_hash);
        assert_ne!(no_address_hash, no_address_no_data_hash);
    }
}
