use std::io::Write;

use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};

use crate::OutputCompressedAccountWithPackedContext;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    pub seq: u64,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub output_leaf_indices: Vec<u32>,
    pub sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compress_or_decompress_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

impl PublicTransactionEvent {
    pub fn man_serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&(self.input_compressed_account_hashes.len() as u32).to_le_bytes())?;
        for hash in self.input_compressed_account_hashes.iter() {
            writer.write_all(hash)?;
        }

        writer.write_all(&(self.output_compressed_account_hashes.len() as u32).to_le_bytes())?;
        for hash in self.output_compressed_account_hashes.iter() {
            writer.write_all(hash)?;
        }

        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
        writer.write_all(&(self.output_compressed_accounts.len() as u32).to_le_bytes())?;
        for i in 0..self.output_compressed_accounts.len() {
            let account = self.output_compressed_accounts[i].clone();
            account.serialize(writer)?;
        }
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos).unwrap();

        writer.write_all(&(self.output_leaf_indices.len() as u32).to_le_bytes())?;
        for index in self.output_leaf_indices.iter() {
            writer.write_all(&index.to_le_bytes())?;
        }

        writer.write_all(&(self.sequence_numbers.len() as u32).to_le_bytes())?;
        for element in self.sequence_numbers.iter() {
            writer.write_all(&element.pubkey.to_bytes())?;
            writer.write_all(&element.seq.to_le_bytes())?;
        }
        match self.relay_fee {
            Some(relay_fee) => {
                writer.write_all(&[1])?;
                writer.write_all(&relay_fee.to_le_bytes())
            }
            None => writer.write_all(&[0]),
        }?;

        writer.write_all(&[self.is_compress as u8])?;

        match self.compress_or_decompress_lamports {
            Some(compress_or_decompress_lamports) => {
                writer.write_all(&[1])?;
                writer.write_all(&compress_or_decompress_lamports.to_le_bytes())
            }
            None => writer.write_all(&[0]),
        }?;

        writer.write_all(&(self.pubkey_array.len() as u32).to_le_bytes())?;
        for pubkey in self.pubkey_array.iter() {
            writer.write_all(&pubkey.to_bytes())?;
        }

        match &self.message {
            Some(message) => {
                writer.write_all(&[1])?;
                writer.write_all(&(message.len() as u32).to_le_bytes())?;
                writer.write_all(message)
            }
            None => writer.write_all(&[0]),
        }?;

        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use rand::Rng;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;
    use crate::sdk::compressed_account::{CompressedAccount, CompressedAccountData};

    #[test]
    fn test_manual_vs_borsh_serialization() {
        // Create a sample `PublicTransactionEvent` instance
        let event = PublicTransactionEvent {
            input_compressed_account_hashes: vec![[0u8; 32], [1u8; 32]],
            output_compressed_account_hashes: vec![[2u8; 32], [3u8; 32]],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Keypair::new().pubkey(),
                    lamports: 100,
                    address: Some([5u8; 32]),
                    data: Some(CompressedAccountData {
                        discriminator: [6u8; 8],
                        data: vec![7u8; 32],
                        data_hash: [8u8; 32],
                    }),
                },
                merkle_tree_index: 1,
            }],
            sequence_numbers: vec![
                MerkleTreeSequenceNumber {
                    pubkey: Keypair::new().pubkey(),
                    seq: 10,
                },
                MerkleTreeSequenceNumber {
                    pubkey: Keypair::new().pubkey(),
                    seq: 2,
                },
            ],
            output_leaf_indices: vec![4, 5, 6],
            relay_fee: Some(1000),
            is_compress: true,
            compress_or_decompress_lamports: Some(5000),
            pubkey_array: vec![Keypair::new().pubkey(), Keypair::new().pubkey()],
            message: Some(vec![8, 9, 10]),
        };

        // Serialize using Borsh
        let borsh_serialized = event.try_to_vec().unwrap();

        // Serialize manually
        let mut manual_serialized = Vec::new();
        event.man_serialize(&mut manual_serialized).unwrap();

        // Compare the two byte arrays
        assert_eq!(
            borsh_serialized, manual_serialized,
            "Borsh and manual serialization results should match"
        );
    }

    #[test]
    fn test_serialization_consistency() {
        let mut rng = rand::thread_rng();

        for _ in 0..10_000 {
            let input_hashes: Vec<[u8; 32]> =
                (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let output_hashes: Vec<[u8; 32]> =
                (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let output_accounts: Vec<OutputCompressedAccountWithPackedContext> = (0..rng
                .gen_range(1..10))
                .map(|_| OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: Keypair::new().pubkey(),
                        lamports: rng.gen(),
                        address: Some(rng.gen()),
                        data: None,
                    },
                    merkle_tree_index: rng.gen(),
                })
                .collect();
            let leaf_indices: Vec<u32> = (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let pubkeys: Vec<Pubkey> = (0..rng.gen_range(1..10))
                .map(|_| Keypair::new().pubkey())
                .collect();
            let message: Option<Vec<u8>> = if rng.gen() {
                Some((0..rng.gen_range(1..100)).map(|_| rng.gen()).collect())
            } else {
                None
            };

            let event = PublicTransactionEvent {
                input_compressed_account_hashes: input_hashes,
                output_compressed_account_hashes: output_hashes,
                output_compressed_accounts: output_accounts,
                output_leaf_indices: leaf_indices,
                sequence_numbers: (0..rng.gen_range(1..10))
                    .map(|_| MerkleTreeSequenceNumber {
                        pubkey: Keypair::new().pubkey(),
                        seq: rng.gen(),
                    })
                    .collect(),
                relay_fee: if rng.gen() { Some(rng.gen()) } else { None },
                is_compress: rng.gen(),
                compress_or_decompress_lamports: if rng.gen() { Some(rng.gen()) } else { None },
                pubkey_array: pubkeys,
                message,
            };

            let borsh_serialized = event.try_to_vec().unwrap();
            let mut manual_serialized = Vec::new();
            event.man_serialize(&mut manual_serialized).unwrap();

            assert_eq!(
                borsh_serialized, manual_serialized,
                "Borsh and manual serialization results should match"
            );
        }
    }
}
