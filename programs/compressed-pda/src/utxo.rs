use std::collections::HashMap;

use anchor_lang::prelude::*;
use light_hasher::{Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_le;

use crate::{
    tlv::{Tlv, TlvSerializable},
    ErrorCode,
};
// there are two sources I can get the pubkey from the transaction object and the other account keys
// the index starts with the accounts keys of the transaction object, if the index is larger than the length of the accounts keys
// we access the pubkey array with our additiona pubkeys

// we need a general macro that just derives a serializable struct from a struct that replaces every pubkey with a u8
// the struct should be borsh serializable and deserializable
// additionally the macro should derive a function that converts the serializable struct back to the original struct with the additional input
// of a slice of account infos where it gets the pubkeys from
// additionally the macro needs to derive a function that converts the original struct into the serializable struct and outputs the struct and the pubkeys
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SerializedUtxos {
    pub pubkey_array: Vec<Pubkey>,
    pub u64_array: Vec<u64>,
    pub in_utxos: Vec<(InUtxoSerializable, u8, u8)>,
    pub out_utxos: Vec<(OutUtxoSerializable, u8)>,
}

impl SerializedUtxos {
    pub fn in_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
        merkle_tree_accounts: &[Pubkey],
    ) -> Result<Vec<(Utxo, u8, u8)>> {
        let mut in_utxos = Vec::with_capacity(self.in_utxos.len());
        for (i, (in_utxo, index_mt_account, index_nullifier_array_account)) in
            self.in_utxos.iter().enumerate()
        {
            let owner = if (in_utxo.owner as usize) < accounts.len() {
                accounts[in_utxo.owner as usize]
            } else {
                self.pubkey_array[in_utxo.owner.saturating_sub(accounts.len() as u8) as usize]
            };
            let lamports = self.u64_array[in_utxo.lamports as usize];
            let data = in_utxo.data.as_ref().map(|data| {
                data.tlv_from_serializable_tlv(
                    [accounts, self.pubkey_array.as_slice()].concat().as_slice(),
                )
            });
            let mut utxo = Utxo {
                owner,
                blinding: [0u8; 32],
                lamports,
                data,
            };
            utxo.update_blinding(merkle_tree_accounts[i].key(), in_utxo.leaf_index as usize)?;
            in_utxos.push((utxo, *index_mt_account, *index_nullifier_array_account));
        }
        Ok(in_utxos)
    }

    pub fn out_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<(OutUtxo, u8)>> {
        let mut out_utxos = Vec::with_capacity(self.out_utxos.len());
        for (out_utxo, index_mt_account) in self.out_utxos.iter() {
            let owner = if (out_utxo.owner as usize) < accounts.len() {
                accounts[out_utxo.owner as usize]
            } else {
                self.pubkey_array[out_utxo.owner.saturating_sub(accounts.len() as u8) as usize]
            };
            let lamports = self.u64_array[out_utxo.lamports as usize];
            let data = out_utxo.data.as_ref().map(|data| {
                data.tlv_from_serializable_tlv(
                    [accounts, self.pubkey_array.as_slice()].concat().as_slice(),
                )
            });
            let utxo = OutUtxo {
                owner,
                lamports,
                data,
            };
            out_utxos.push((utxo, *index_mt_account));
        }
        Ok(out_utxos)
    }

    pub fn add_in_utxos(
        &mut self,
        utxos_to_add: &[Utxo],
        accounts: &[Pubkey],
        leaf_indices: &[u32],
        in_utxo_merkle_tree_pubkeys: &[Pubkey],
        nullifier_array_pubkeys: &[Pubkey],
    ) -> Result<()> {
        if !self.in_utxos.is_empty() {
            return Err(ErrorCode::InUtxosAlreadyAdded.into());
        }
        if utxos_to_add.len() != leaf_indices.len() {
            return Err(ErrorCode::NumberOfLeavesMissmatch.into());
        }
        if utxos_to_add.len() != in_utxo_merkle_tree_pubkeys.len() {
            return Err(ErrorCode::MerkleTreePubkeysMissmatch.into());
        }
        if utxos_to_add.len() != nullifier_array_pubkeys.len() {
            return Err(ErrorCode::NullifierArrayPubkeysMissmatch.into());
        }
        let mut utxos = Vec::with_capacity(utxos_to_add.len());
        for (i, utxo) in utxos_to_add.iter().enumerate() {
            // Determine the owner index
            let owner_index = match accounts.iter().position(|&p| p == utxo.owner) {
                Some(index) => index as u8, // Found in accounts
                None => match self.pubkey_array.iter().position(|&p| p == utxo.owner) {
                    Some(index) => (accounts.len() + index) as u8, // Found in accounts
                    None => {
                        // Not found, add to pubkey_array and use index
                        self.pubkey_array.push(utxo.owner);
                        (accounts.len() + self.pubkey_array.len() - 1) as u8
                    }
                },
            };

            // Add the lamports index
            let lamports_index = match self.u64_array.iter().position(|&p| p == utxo.lamports) {
                Some(index) => index as u8, // Found in accounts
                None => {
                    // Not found, add to u64_array and use index
                    self.u64_array.push(utxo.lamports);
                    (self.u64_array.len() - 1) as u8
                }
            };

            // Serialize the UTXO data, if present
            let data_serializable = utxo.data.as_ref().map(|data| {
                // This transformation needs to be defined based on how Tlv can be converted to TlvSerializable
                Tlv::to_serializable_tlv(data, &mut self.pubkey_array, accounts)
            });

            // Create and add the InUtxoSerializable
            let in_utxo_serializable = InUtxoSerializable {
                owner: owner_index,
                leaf_index: leaf_indices[i],
                lamports: lamports_index,
                data: data_serializable,
            };
            utxos.push((in_utxo_serializable, 0u8, 0u8));
        }
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        for (i, mt) in in_utxo_merkle_tree_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i);
                }
            };
            utxos[i].1 = *remaining_accounts.get(mt).unwrap() as u8;
        }
        let len: usize = remaining_accounts.len();
        for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            utxos[i].2 = *remaining_accounts.get(mt).unwrap() as u8;
        }
        self.in_utxos.extend(utxos);
        Ok(())
    }

    pub fn add_out_utxos(
        &mut self,
        utxos_to_add: &[OutUtxo],
        accounts: &[Pubkey],
        remaining_accounts_pubkeys: &[Pubkey],
        out_utxo_merkle_tree_pubkeys: &[Pubkey],
    ) -> Result<()> {
        let mut utxos = Vec::with_capacity(utxos_to_add.len());
        for utxo in utxos_to_add.iter() {
            // Determine the owner index
            let owner_index = match accounts.iter().position(|&p| p == utxo.owner) {
                Some(index) => index as u8, // Found in accounts
                None => match self.pubkey_array.iter().position(|&p| p == utxo.owner) {
                    Some(index) => (accounts.len() + index) as u8, // Found in accounts
                    None => {
                        // Not found, add to pubkey_array and use index
                        self.pubkey_array.push(utxo.owner);
                        (accounts.len() + self.pubkey_array.len() - 1) as u8
                    }
                },
            };

            // Add the lamports index
            let lamports_index = match self.u64_array.iter().position(|&p| p == utxo.lamports) {
                Some(index) => index as u8, // Found in accounts
                None => {
                    // Not found, add to u64_array and use index
                    self.u64_array.push(utxo.lamports);
                    (self.u64_array.len() - 1) as u8
                }
            };

            // Serialize the UTXO data, if present
            let data_serializable = utxo.data.as_ref().map(|data| {
                // This transformation needs to be defined based on how Tlv can be converted to TlvSerializable
                Tlv::to_serializable_tlv(data, &mut self.pubkey_array, accounts)
            });

            // Create and add the InUtxoSerializable
            let in_utxo_serializable = OutUtxoSerializable {
                owner: owner_index,
                lamports: lamports_index,
                data: data_serializable,
            };
            utxos.push((in_utxo_serializable, 0u8));
        }
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        remaining_accounts_pubkeys
            .iter()
            .enumerate()
            .for_each(|(i, pubkey)| {
                remaining_accounts.insert(*pubkey, i);
            });
        let len = remaining_accounts.len();
        for (i, mt) in out_utxo_merkle_tree_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            utxos[i].1 = *remaining_accounts.get(mt).unwrap() as u8;
        }
        self.out_utxos.extend(utxos);
        Ok(())
    }
}

// #[account]
#[derive(Debug, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InUtxoSerializable {
    pub owner: u8,
    pub leaf_index: u32,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

// no need to send blinding is computed onchain
// #[account]
#[derive(Debug, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OutUtxoSerializable {
    pub owner: u8,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

// #[account]
#[derive(Debug, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OutUtxo {
    pub owner: Pubkey,
    pub lamports: u64,
    pub data: Option<Tlv>,
}

// blinding we just need to send the leafIndex
// #[account]
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Utxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<Tlv>,
}

impl Utxo {
    pub fn update_blinding(&mut self, merkle_tree_pda: Pubkey, index_of_leaf: usize) -> Result<()> {
        self.blinding = Poseidon::hashv(&[
            &hash_to_bn254_field_size_le(&merkle_tree_pda.to_bytes())
                .unwrap()  
                .0,
            index_of_leaf.to_le_bytes().as_slice(),
        ])
        .unwrap();
        Ok(())
    }

    pub fn hash(&self) -> [u8; 32] {
        let tlv_data_hash = match &self.data {
            Some(data) => {
                let hashes = data
                    .tlv_elements
                    .iter()
                    .map(|d| d.data_hash.as_slice())
                    .collect::<Vec<&[u8]>>();
                Poseidon::hashv(hashes.as_slice()).unwrap()
            }
            None => [0u8; 32],
        };
        Poseidon::hashv(&[
            &hash_to_bn254_field_size_le(&self.owner.to_bytes())
                .unwrap()
                .0,
            &self.blinding,
            &self.lamports.to_le_bytes(),
            &tlv_data_hash,
        ])
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_add_in_utxos() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

        let owner_pubkey = Pubkey::new_unique();
        let owner2_pubkey = Pubkey::new_unique();

        let accounts = vec![owner_pubkey];
        let utxo = Utxo {
            owner: owner_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: None,
        };
        let utxo_1 = Utxo {
            owner: owner2_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: None,
        };
        let utxo_2 = Utxo {
            owner: owner2_pubkey,
            blinding: [0u8; 32],
            lamports: 201,
            data: None,
        };
        let merkle_tree_pubkey_0 = Pubkey::new_unique();
        let merkle_tree_pubkey_1 = Pubkey::new_unique();
        let nullifier_array_pubkey_0 = Pubkey::new_unique();
        let nullifier_array_pubkey_1 = Pubkey::new_unique();
        let in_utxo_merkle_tree_pubkeys = vec![
            merkle_tree_pubkey_0,
            merkle_tree_pubkey_1,
            merkle_tree_pubkey_0,
        ];
        let nullifier_array_pubkeys = vec![
            nullifier_array_pubkey_0,
            nullifier_array_pubkey_1,
            nullifier_array_pubkey_1,
        ];
        serialized_utxos
            .add_in_utxos(
                &[utxo, utxo_1, utxo_2],
                &accounts,
                &[0, 1, 2],
                &in_utxo_merkle_tree_pubkeys,
                &nullifier_array_pubkeys,
            )
            .unwrap();

        assert_eq!(serialized_utxos.in_utxos.len(), 3);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.in_utxos[0].0,
            InUtxoSerializable {
                owner: 0,
                leaf_index: 0,
                lamports: 0,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.in_utxos[0].1, 0);
        assert_eq!(serialized_utxos.in_utxos[0].2, 2);

        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.in_utxos[1].0,
            InUtxoSerializable {
                owner: 1,
                leaf_index: 1,
                lamports: 0,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.in_utxos[1].1, 1);
        assert_eq!(serialized_utxos.in_utxos[1].2, 3);

        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 2);
        assert_eq!(serialized_utxos.u64_array[1], 201);
        assert_eq!(
            serialized_utxos.in_utxos[2].0,
            InUtxoSerializable {
                owner: 1,
                leaf_index: 2,
                lamports: 1,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.in_utxos[2].1, 0);
        assert_eq!(serialized_utxos.in_utxos[2].2, 3);
    }

    #[test]
    fn test_add_out_utxos() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };
        let merkle_tree_pubkey_0 = Pubkey::new_unique();
        let merkle_tree_pubkey_1 = Pubkey::new_unique();
        let nullifier_array_pubkey_1 = Pubkey::new_unique();
        let out_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_1];
        let remaining_accounts_pubkeys = vec![
            merkle_tree_pubkey_0,
            nullifier_array_pubkey_1,
            merkle_tree_pubkey_1,
        ];
        let owner_pubkey = Pubkey::new_unique();
        let owner2_pubkey = Pubkey::new_unique();

        let accounts = vec![owner_pubkey];
        let utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: None,
        };

        serialized_utxos
            .add_out_utxos(
                &[utxo],
                &accounts,
                &remaining_accounts_pubkeys,
                &out_utxo_merkle_tree_pubkeys,
            )
            .unwrap();

        assert_eq!(serialized_utxos.out_utxos.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array.len(), 0);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.out_utxos[0].0,
            OutUtxoSerializable {
                owner: 0,
                lamports: 0,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.out_utxos[0].1, 2);

        let utxo = OutUtxo {
            owner: owner2_pubkey,
            lamports: 100,
            data: None,
        };

        serialized_utxos
            .add_out_utxos(
                &[utxo],
                &accounts,
                &remaining_accounts_pubkeys,
                &vec![merkle_tree_pubkey_0],
            )
            .unwrap();
        assert_eq!(serialized_utxos.out_utxos.len(), 2);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.out_utxos[1].0,
            OutUtxoSerializable {
                owner: 1,
                lamports: 0,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.out_utxos[1].1, 0);

        let utxo = OutUtxo {
            owner: owner2_pubkey,
            lamports: 201,
            data: None,
        };

        serialized_utxos
            .add_out_utxos(
                &[utxo],
                &accounts,
                &remaining_accounts_pubkeys,
                &out_utxo_merkle_tree_pubkeys,
            )
            .unwrap();
        assert_eq!(serialized_utxos.out_utxos.len(), 3);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 2);
        assert_eq!(serialized_utxos.u64_array[1], 201);
        assert_eq!(
            serialized_utxos.out_utxos[2].0,
            OutUtxoSerializable {
                owner: 1,
                lamports: 1,
                data: None,
            }
        );
        assert_eq!(serialized_utxos.out_utxos[1].1, 0);
    }

    #[test]
    fn test_add_in_and_out_utxos() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };
        let merkle_tree_pubkey_0 = Pubkey::new_unique();
        let merkle_tree_pubkey_1 = Pubkey::new_unique();
        let nullifier_array_pubkey_0 = Pubkey::new_unique();
        let nullifier_array_pubkey_1 = Pubkey::new_unique();
        let in_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_0];
        let nullifier_array_pubkeys = vec![nullifier_array_pubkey_0];

        let owner_pubkey = Pubkey::new_unique();
        let owner2_pubkey = Pubkey::new_unique();
        let accounts = vec![owner_pubkey];

        // Adding an InUtxo
        let in_utxo = Utxo {
            owner: owner_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: None,
        };

        serialized_utxos
            .add_in_utxos(
                &[in_utxo.clone()],
                &accounts,
                &[0],
                &in_utxo_merkle_tree_pubkeys,
                &nullifier_array_pubkeys,
            )
            .unwrap();

        // Adding an OutUtxo with the same owner
        let out_utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: None,
        };
        let out_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_1];
        let remaining_accounts_pubkeys = vec![
            merkle_tree_pubkey_0,
            nullifier_array_pubkey_1,
            merkle_tree_pubkey_1,
        ];
        serialized_utxos
            .add_out_utxos(
                &[out_utxo.clone()],
                &accounts,
                &remaining_accounts_pubkeys,
                &out_utxo_merkle_tree_pubkeys,
            )
            .unwrap();

        // Adding another OutUtxo with a different owner
        let out_utxo2 = OutUtxo {
            owner: owner2_pubkey,
            lamports: 200,
            data: None,
        };
        let out_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_0];

        serialized_utxos
            .add_out_utxos(
                &[out_utxo2.clone()],
                &accounts,
                &remaining_accounts_pubkeys,
                &out_utxo_merkle_tree_pubkeys,
            )
            .unwrap();

        // Assertions for InUtxo
        assert_eq!(serialized_utxos.in_utxos.len(), 1);
        assert!(serialized_utxos.in_utxos.iter().any(
            |(u, index_merkle_tree_pubkey, index_nullifier_array_pubkey)| u.owner == 0
                && u.lamports == 0
                && u.leaf_index == 0
                && u.data.is_none()
                && *index_merkle_tree_pubkey == 0
                && *index_nullifier_array_pubkey == 1
        ));

        // Assertions for OutUtxo
        assert_eq!(serialized_utxos.out_utxos.len(), 2);
        assert!(serialized_utxos
            .out_utxos
            .iter()
            .any(|(u, index_merkle_tree_pubkey)| u.owner == 0
                && u.lamports == 0
                && u.data.is_none()
                && *index_merkle_tree_pubkey == 2));
        assert!(serialized_utxos
            .out_utxos
            .iter()
            .any(|(u, index_merkle_tree_pubkey)| u.owner == 1
                && u.lamports == 1
                && u.data.is_none()
                && *index_merkle_tree_pubkey == 0));
        // Checking pubkey_array and u64_array
        assert_eq!(
            serialized_utxos.pubkey_array.len(),
            1,
            "Should contain exactly one additional pubkey"
        );
        assert_eq!(
            serialized_utxos.pubkey_array[0], owner2_pubkey,
            "The additional pubkey should match owner2_pubkey"
        );
        assert_eq!(
            serialized_utxos.u64_array.len(),
            2,
            "Should contain exactly two unique lamport values"
        );
        assert_eq!(
            serialized_utxos.u64_array[serialized_utxos.out_utxos[0].0.lamports as usize], 100,
            "Should contain lamports value 100"
        );
        assert_eq!(
            serialized_utxos.u64_array[serialized_utxos.out_utxos[1].0.lamports as usize], 200,
            "Should contain lamports value 200"
        );
        let merkle_tree_accounts = vec![Pubkey::new_unique(), Pubkey::new_unique()]; // Mocked merkle tree accounts for blinding computation
        let deserialized_in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &merkle_tree_accounts)
            .unwrap();

        // Deserialization step for OutUtxos
        // Assuming out_utxos_from_serialized_utxos method exists and works similarly to in_utxos_from_serialized_utxos
        let deserialized_out_utxos = serialized_utxos
            .out_utxos_from_serialized_utxos(&accounts)
            .unwrap();

        // Assertions for deserialized InUtxos
        assert_eq!(deserialized_in_utxos.len(), 1);
        assert_eq!(deserialized_in_utxos[0].0.owner, in_utxo.owner);
        assert_eq!(deserialized_in_utxos[0].0.lamports, in_utxo.lamports);
        assert_eq!(deserialized_in_utxos[0].0.data, None);
        let out_utxos = vec![out_utxo, out_utxo2];
        // Assertions for deserialized OutUtxos
        assert_eq!(deserialized_out_utxos.len(), 2);
        deserialized_out_utxos
            .iter()
            .enumerate()
            .for_each(|(i, (u, _))| {
                assert!(
                    u.owner == out_utxos[i].owner
                        && u.lamports == out_utxos[i].lamports
                        && u.data == out_utxos[i].data
                )
            });
    }

    #[test]
    fn test_in_utxos_from_serialized_utxos() {
        let owner_pubkey = Pubkey::new_unique();
        let merkle_tree_account = Pubkey::new_unique();
        let serialized_utxos = SerializedUtxos {
            pubkey_array: vec![owner_pubkey],
            u64_array: vec![100],
            in_utxos: vec![(
                InUtxoSerializable {
                    owner: 0,
                    leaf_index: 1,
                    lamports: 0,
                    data: None,
                },
                0,
                0,
            )],
            out_utxos: vec![],
        };

        let accounts = vec![]; // No additional accounts needed for this test
        let merkle_tree_accounts = vec![merkle_tree_account];

        let in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &merkle_tree_accounts)
            .unwrap();

        assert_eq!(in_utxos.len(), 1);
        let utxo = &in_utxos[0];
        assert_eq!(utxo.0.owner, owner_pubkey);
        assert_eq!(utxo.0.lamports, 100);
    }
}
