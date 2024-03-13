use anchor_lang::prelude::*;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TlvSerializable {
    pub tlv_elements: Vec<TlvDataElementSerializable>,
}

impl TlvSerializable {
    pub fn tlv_from_serializable_tlv(&self, accounts: &[Pubkey]) -> Tlv {
        let mut tlv_elements = Vec::with_capacity(self.tlv_elements.len());
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
mod test {
    use super::*;
    use crate::utxo::{SerializedUtxos, Utxo};

    #[test]
    fn test_to_serializable_tlv() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique(); // This pubkey will simulate an "external" pubkey not initially in accounts.
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
        let pubkey1 = Pubkey::new_unique();
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
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
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
        let merkle_tree_pubkey_0 = Pubkey::new_unique();
        let nullifier_array_pubkey_0 = Pubkey::new_unique();
        let in_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_0];
        let nullifier_array_pubkeys = vec![nullifier_array_pubkey_0];
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
            address: None,
            data: Some(tlv_data),
        };

        // Assuming add_in_utxos is modified to accept UTXOs with TLV data correctly
        serialized_utxos
            .add_in_utxos(
                &[utxo],
                &accounts,
                &[1],
                &in_utxo_merkle_tree_pubkeys,
                &nullifier_array_pubkeys,
            )
            .unwrap();

        assert_eq!(
            serialized_utxos.in_utxos.len(),
            1,
            "Should have added one UTXO"
        );
        assert!(
            serialized_utxos.in_utxos[0]
                .in_utxo_serializable
                .data
                .is_some(),
            "UTXO should have TLV data"
        );

        // Verify that TLV data was serialized correctly
        let serialized_tlv_data = serialized_utxos.in_utxos[0]
            .in_utxo_serializable
            .data
            .as_ref()
            .unwrap();
        assert_eq!(
            *serialized_tlv_data, tlv_serializable,
            "TLV data should match the serialized version"
        );
    }
}
