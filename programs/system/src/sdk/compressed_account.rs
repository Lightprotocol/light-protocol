use std::collections::HashMap;

use anchor_lang::prelude::*;
use light_hasher::{Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_be;

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}
impl CompressedAccountWithMerkleContext {
    pub fn hash(&self) -> Result<[u8; 32]> {
        self.compressed_account.hash::<Poseidon>(
            &self.merkle_context.merkle_tree_pubkey,
            &self.merkle_context.leaf_index,
        )
    }
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    /// Index of leaf in queue. Placeholder of batched Merkle tree updates
    /// currently unimplemented.
    pub queue_index: Option<QueueIndex>,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    /// Index of leaf in queue. Placeholder of batched Merkle tree updates
    /// currently unimplemented.
    pub queue_index: Option<QueueIndex>,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct QueueIndex {
    /// Id of queue in queue account.
    pub queue_id: u8,
    /// Index of compressed account hash in queue.
    pub index: u16,
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
            queue_index: None,
        })
        .collect::<Vec<PackedMerkleContext>>();
    let mut index: usize = remaining_accounts.len();
    for (i, params) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&params.merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.merkle_tree_pubkey, index);
                index += 1;
            }
        };
        merkle_context_packed[i].merkle_tree_pubkey_index =
            *remaining_accounts.get(&params.merkle_tree_pubkey).unwrap() as u8;
    }

    for (i, params) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&params.nullifier_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.nullifier_queue_pubkey, index);
                index += 1;
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

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl CompressedAccount {
    pub fn hash_with_hashed_values<H: Hasher>(
        &self,
        &owner_hashed: &[u8; 32],
        &merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
    ) -> Result<[u8; 32]> {
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
        let hash = H::hashv(&vec).map_err(ProgramError::from)?;
        Ok(hash)
    }

    pub fn hash<H: Hasher>(
        &self,
        &merkle_tree_pubkey: &Pubkey,
        leaf_index: &u32,
    ) -> Result<[u8; 32]> {
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
    use solana_sdk::signature::{Keypair, Signer};

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
        let owner = Keypair::new().pubkey();
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
        let merkle_tree_pubkey = Keypair::new().pubkey();
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
