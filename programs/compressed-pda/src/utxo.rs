use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hash, pubkey::Pubkey},
};
use light_hasher::{Hasher, Poseidon};

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
    pub in_utxos: Vec<InUtxoSerializable>,
    pub out_utxos: Vec<OutUtxoSerializable>,
}

impl SerializedUtxos {
    pub fn in_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
        merkle_tree_accounts: &[Pubkey],
    ) -> Result<Vec<Utxo>> {
        let mut in_utxos = Vec::new();
        for (i, in_utxo) in self.in_utxos.iter().enumerate() {
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
            in_utxos.push(utxo);
        }
        Ok(in_utxos)
    }

    pub fn out_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
        merkle_tree_accounts: &[Pubkey],
        leaf_indices: &[u32],
    ) -> Result<Vec<Utxo>> {
        let mut out_utxos = Vec::new();
        for (i, out_utxo) in self.out_utxos.iter().enumerate() {
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
            let mut utxo = Utxo {
                owner,
                blinding: [0u8; 32],
                lamports,
                data,
            };
            utxo.update_blinding(merkle_tree_accounts[i].key(), leaf_indices[i] as usize)?;
            out_utxos.push(utxo);
        }
        Ok(out_utxos)
    }

    pub fn add_in_utxos(
        &mut self,
        utxos_to_add: &[Utxo],
        accounts: &[Pubkey],
        leaf_indices: &[u32],
    ) -> Result<()> {
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
            self.in_utxos.push(in_utxo_serializable);
        }
        Ok(())
    }

    pub fn add_out_utxos(&mut self, utxos_to_add: &[OutUtxo], accounts: &[Pubkey]) -> Result<()> {
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
            self.out_utxos.push(in_utxo_serializable);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
#[account]
pub struct InUtxoSerializable {
    pub owner: u8,
    pub leaf_index: u32,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

// no need to send blinding is computed onchain
#[derive(Debug, PartialEq)]
#[account]
pub struct OutUtxoSerializable {
    pub owner: u8,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

#[derive(Debug)]
#[account]
pub struct OutUtxo {
    pub owner: Pubkey,
    pub lamports: u64,
    pub data: Option<Tlv>,
}

// blinding we just need to send the leafIndex
#[derive(Debug, PartialEq)]
#[account]
pub struct Utxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<Tlv>,
}

impl Utxo {
    pub fn update_blinding(&mut self, merkle_tree_pda: Pubkey, index_of_leaf: usize) -> Result<()> {
        self.blinding = Poseidon::hashv(&[
            &hash(merkle_tree_pda.to_bytes().as_slice()).to_bytes()[0..30],
            index_of_leaf.to_le_bytes().as_slice(),
        ])
        .unwrap();
        Ok(())
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TlvSerializable {
    pub tlv_elements: Vec<TlvDataElementSerializable>,
}

impl TlvSerializable {
    pub fn tlv_from_serializable_tlv(&self, accounts: &[Pubkey]) -> Tlv {
        let mut tlv_elements = Vec::new();
        for tlv_element in &self.tlv_elements {
            let owner = accounts[tlv_element.owner as usize];
            tlv_elements.push(TlvDataElement {
                discriminator: tlv_element.discriminator,
                owner,
                data: tlv_element.data.clone(),
                data_hash: tlv_element.data_hash,
            });
        }
        Tlv { tlv_elements }
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct Tlv {
    pub tlv_elements: Vec<TlvDataElement>,
}

impl Tlv {
    pub fn to_serializable_tlv(
        &self,
        pubkey_array: &mut Vec<Pubkey>,
        accounts: &[Pubkey],
    ) -> TlvSerializable {
        let mut tlv_elements_serializable = Vec::new();

        for tlv_element in &self.tlv_elements {
            // Try to find the owner in the accounts vector.
            let owner_index = match accounts.iter().position(|&p| p == tlv_element.owner) {
                Some(index) => index as u8, // Owner found, use existing index
                None => match pubkey_array.iter().position(|&p| p == tlv_element.owner) {
                    Some(index) => (accounts.len() + index) as u8, // Owner found, use existing index
                    None => {
                        // Owner not found, append to accounts and use new index
                        pubkey_array.push(tlv_element.owner);
                        (accounts.len() + pubkey_array.len() - 1) as u8
                    }
                },
            };

            let serializable_element = TlvDataElementSerializable {
                discriminator: tlv_element.discriminator,
                owner: owner_index,
                data: tlv_element.data.clone(),
                data_hash: tlv_element.data_hash,
            };

            tlv_elements_serializable.push(serializable_element);
        }

        TlvSerializable {
            tlv_elements: tlv_elements_serializable,
        }
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TlvDataElementSerializable {
    pub discriminator: [u8; 8],
    pub owner: u8,
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

/// Time lock escrow example:
/// escrow tlv data -> compressed token program
/// let escrow_data = {
///   owner: Pubkey, // owner is the user pubkey
///   release_slot: u64,
///   deposit_slot: u64,
/// };
///
/// let escrow_tlv_data = TlvDataElement {
///   discriminator: [1,0,0,0,0,0,0,0],
///   owner: escrow_program_id,
///   data: escrow_data.try_to_vec()?,
/// };
/// let token_tlv = TlvDataElement {
///   discriminator: [2,0,0,0,0,0,0,0],
///   owner: token_program,
///   data: token_data.try_to_vec()?,
/// };
/// let token_data = Account {
///  mint,
///  owner,
///  amount: 10_000_000u64,
///  delegate: None,
///  state: Initialized, (u64)
///  is_native: None,
///  delegated_amount: 0u64,
///  close_authority: None,
/// };
///
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TlvDataElement {
    pub discriminator: [u8; 8],
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
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

        serialized_utxos
            .add_in_utxos(&[utxo], &accounts, &[0])
            .unwrap();

        assert_eq!(serialized_utxos.in_utxos.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array.len(), 0);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.in_utxos[0],
            InUtxoSerializable {
                owner: 0,
                leaf_index: 0,
                lamports: 0,
                data: None,
            }
        );
        let utxo = Utxo {
            owner: owner2_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: None,
        };

        serialized_utxos
            .add_in_utxos(&[utxo], &accounts, &[1])
            .unwrap();
        assert_eq!(serialized_utxos.in_utxos.len(), 2);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.in_utxos[1],
            InUtxoSerializable {
                owner: 1,
                leaf_index: 1,
                lamports: 0,
                data: None,
            }
        );

        let utxo = Utxo {
            owner: owner2_pubkey,
            blinding: [0u8; 32],
            lamports: 201,
            data: None,
        };

        serialized_utxos
            .add_in_utxos(&[utxo], &accounts, &[2])
            .unwrap();
        assert_eq!(serialized_utxos.in_utxos.len(), 3);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 2);
        assert_eq!(serialized_utxos.u64_array[1], 201);
        assert_eq!(
            serialized_utxos.in_utxos[2],
            InUtxoSerializable {
                owner: 1,
                leaf_index: 2,
                lamports: 1,
                data: None,
            }
        );
    }

    #[test]
    fn test_add_out_utxos() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

        let owner_pubkey = Pubkey::new_unique();
        let owner2_pubkey = Pubkey::new_unique();

        let accounts = vec![owner_pubkey];
        let utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: None,
        };

        serialized_utxos.add_out_utxos(&[utxo], &accounts).unwrap();

        assert_eq!(serialized_utxos.out_utxos.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array.len(), 0);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.out_utxos[0],
            OutUtxoSerializable {
                owner: 0,
                lamports: 0,
                data: None,
            }
        );
        let utxo = OutUtxo {
            owner: owner2_pubkey,
            lamports: 100,
            data: None,
        };

        serialized_utxos.add_out_utxos(&[utxo], &accounts).unwrap();
        assert_eq!(serialized_utxos.out_utxos.len(), 2);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 1);
        assert_eq!(serialized_utxos.u64_array[0], 100);
        assert_eq!(
            serialized_utxos.out_utxos[1],
            OutUtxoSerializable {
                owner: 1,
                lamports: 0,
                data: None,
            }
        );

        let utxo = OutUtxo {
            owner: owner2_pubkey,
            lamports: 201,
            data: None,
        };

        serialized_utxos.add_out_utxos(&[utxo], &accounts).unwrap();
        assert_eq!(serialized_utxos.out_utxos.len(), 3);
        assert_eq!(serialized_utxos.pubkey_array.len(), 1);
        assert_eq!(serialized_utxos.pubkey_array[0], owner2_pubkey);
        assert_eq!(serialized_utxos.u64_array.len(), 2);
        assert_eq!(serialized_utxos.u64_array[1], 201);
        assert_eq!(
            serialized_utxos.out_utxos[2],
            OutUtxoSerializable {
                owner: 1,
                lamports: 1,
                data: None,
            }
        );
    }

    #[test]
    fn test_add_in_and_out_utxos() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

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
            .add_in_utxos(&[in_utxo.clone()], &accounts, &[0])
            .unwrap();

        // Adding an OutUtxo with the same owner
        let out_utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: None,
        };

        serialized_utxos
            .add_out_utxos(&[out_utxo.clone()], &accounts)
            .unwrap();

        // Adding another OutUtxo with a different owner
        let out_utxo2 = OutUtxo {
            owner: owner2_pubkey,
            lamports: 200,
            data: None,
        };

        serialized_utxos
            .add_out_utxos(&[out_utxo2.clone()], &accounts)
            .unwrap();

        // Assertions for InUtxo
        assert_eq!(serialized_utxos.in_utxos.len(), 1);
        assert!(serialized_utxos
            .in_utxos
            .iter()
            .any(|u| u.owner == 0 && u.lamports == 0 && u.leaf_index == 0 && u.data.is_none()));

        // Assertions for OutUtxo
        assert_eq!(serialized_utxos.out_utxos.len(), 2);
        assert!(serialized_utxos
            .out_utxos
            .iter()
            .any(|u| u.owner == 0 && u.lamports == 0 && u.data.is_none()));
        assert!(serialized_utxos
            .out_utxos
            .iter()
            .any(|u| u.owner == 1 && u.lamports == 1 && u.data.is_none()));
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
            serialized_utxos.u64_array[serialized_utxos.out_utxos[0].lamports as usize], 100,
            "Should contain lamports value 100"
        );
        assert_eq!(
            serialized_utxos.u64_array[serialized_utxos.out_utxos[1].lamports as usize], 200,
            "Should contain lamports value 200"
        );
        let merkle_tree_accounts = vec![Pubkey::new_unique(), Pubkey::new_unique()]; // Mocked merkle tree accounts for blinding computation
        let deserialized_in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &merkle_tree_accounts)
            .unwrap();

        // Deserialization step for OutUtxos
        // Assuming out_utxos_from_serialized_utxos method exists and works similarly to in_utxos_from_serialized_utxos
        let leaf_indices: Vec<u32> = vec![2, 3]; // Mocked leaf indices for out_utxos
        let deserialized_out_utxos = serialized_utxos
            .out_utxos_from_serialized_utxos(&accounts, &merkle_tree_accounts, &leaf_indices)
            .unwrap();

        // Assertions for deserialized InUtxos
        assert_eq!(deserialized_in_utxos.len(), 1);
        assert_eq!(deserialized_in_utxos[0].owner, in_utxo.owner);
        assert_eq!(deserialized_in_utxos[0].lamports, in_utxo.lamports);
        assert_eq!(deserialized_in_utxos[0].data, None);
        let out_utxos = vec![out_utxo, out_utxo2];
        // Assertions for deserialized OutUtxos
        assert_eq!(deserialized_out_utxos.len(), 2);
        deserialized_out_utxos
            .iter()
            .enumerate()
            .for_each(|(i, u)| {
                println!("u {:?}", u);
                println!("out_utxo {:?}", out_utxos[i]);
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
            in_utxos: vec![InUtxoSerializable {
                owner: 0,
                leaf_index: 1,
                lamports: 0,
                data: None,
            }],
            out_utxos: vec![],
        };

        let accounts = vec![]; // No additional accounts needed for this test
        let merkle_tree_accounts = vec![merkle_tree_account];

        let in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &merkle_tree_accounts)
            .unwrap();

        assert_eq!(in_utxos.len(), 1);
        let utxo = &in_utxos[0];
        assert_eq!(utxo.owner, owner_pubkey);
        assert_eq!(utxo.lamports, 100);
    }

    fn generate_pubkey() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_to_serializable_tlv() {
        let pubkey1 = generate_pubkey();
        let pubkey2 = generate_pubkey(); // This pubkey will simulate an "external" pubkey not initially in accounts.
        let accounts = vec![pubkey1];
        let mut pubkey_array = Vec::new();

        let tlv = Tlv {
            tlv_elements: vec![
                TlvDataElement {
                    discriminator: [0; 8],
                    owner: pubkey1,
                    data: vec![1, 2, 3],
                    data_hash: [1; 32],
                },
                TlvDataElement {
                    discriminator: [1; 8],
                    owner: pubkey2,
                    data: vec![4, 5, 6],
                    data_hash: [2; 32],
                },
            ],
        };

        let serializable = tlv.to_serializable_tlv(&mut pubkey_array, &accounts);

        // Verify that pubkey_array was updated correctly
        assert_eq!(pubkey_array, vec![pubkey2]);

        // Verify the transformation
        assert_eq!(serializable.tlv_elements.len(), 2);
        assert_eq!(serializable.tlv_elements[0].owner, 0);
        assert_eq!(serializable.tlv_elements[1].owner, 1);
    }

    #[test]
    fn test_to_serializable_tlv_same_owner() {
        let pubkey1 = generate_pubkey();
        let accounts = vec![pubkey1];
        let mut pubkey_array = Vec::new();

        let tlv = Tlv {
            tlv_elements: vec![
                TlvDataElement {
                    discriminator: [0; 8],
                    owner: pubkey1,
                    data: vec![1, 2, 3],
                    data_hash: [1; 32],
                },
                TlvDataElement {
                    discriminator: [1; 8],
                    owner: pubkey1,
                    data: vec![4, 5, 6],
                    data_hash: [2; 32],
                },
            ],
        };

        let serializable = tlv.to_serializable_tlv(&mut pubkey_array, &accounts);

        // Verify that pubkey_array was updated correctly
        assert_eq!(pubkey_array, Vec::new());

        // Verify the transformation
        assert_eq!(serializable.tlv_elements.len(), 2);
        assert_eq!(serializable.tlv_elements[0].owner, 0);
        assert_eq!(serializable.tlv_elements[1].owner, 0);
        let tlv_deserialized = serializable.tlv_from_serializable_tlv(&accounts);
        assert_eq!(tlv, tlv_deserialized);
    }

    #[test]
    fn test_tlv_from_serializable_tlv() {
        let pubkey1 = generate_pubkey();
        let pubkey2 = generate_pubkey();
        let accounts = vec![pubkey1, pubkey2];

        let serializable = TlvSerializable {
            tlv_elements: vec![
                TlvDataElementSerializable {
                    discriminator: [0; 8],
                    owner: 0,
                    data: vec![1, 2, 3],
                    data_hash: [1; 32],
                },
                TlvDataElementSerializable {
                    discriminator: [1; 8],
                    owner: 1,
                    data: vec![4, 5, 6],
                    data_hash: [2; 32],
                },
            ],
        };

        let tlv = serializable.tlv_from_serializable_tlv(&accounts);

        // Verify reconstruction
        assert_eq!(tlv.tlv_elements.len(), 2);
        assert_eq!(tlv.tlv_elements[0].owner, pubkey1);
        assert_eq!(tlv.tlv_elements[1].owner, pubkey2);
    }

    #[test]
    fn test_add_in_utxos_with_tlv_data() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

        let owner_pubkey = Pubkey::new_unique();
        let accounts = vec![owner_pubkey];

        // Creating TLV data for the UTXO
        let tlv_data = Tlv {
            tlv_elements: vec![TlvDataElement {
                discriminator: [1; 8],
                owner: owner_pubkey,
                data: vec![10, 20, 30],
                data_hash: [2; 32],
            }],
        };

        // Convert TLV data to a serializable format
        let mut pubkey_array_for_tlv = Vec::new();
        let tlv_serializable = tlv_data.to_serializable_tlv(&mut pubkey_array_for_tlv, &accounts);

        let utxo = Utxo {
            owner: owner_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: Some(tlv_data),
        };

        // Assuming add_in_utxos is modified to accept UTXOs with TLV data correctly
        serialized_utxos
            .add_in_utxos(&[utxo], &accounts, &[1])
            .unwrap();

        assert_eq!(
            serialized_utxos.in_utxos.len(),
            1,
            "Should have added one UTXO"
        );
        assert!(
            serialized_utxos.in_utxos[0].data.is_some(),
            "UTXO should have TLV data"
        );

        // Verify that TLV data was serialized correctly
        let serialized_tlv_data = serialized_utxos.in_utxos[0].data.as_ref().unwrap();
        assert_eq!(
            *serialized_tlv_data, tlv_serializable,
            "TLV data should match the serialized version"
        );
    }
}
