use crate::{compressed_account::CompressedAccount, InstructionDataTransfer, TransferInstruction};
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use std::{io::Write, mem, str::FromStr};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub output_leaf_indices: Vec<u32>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compression_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

pub trait SizedEvent {
    fn event_size(&self) -> usize;
}

impl SizedEvent for PublicTransactionEvent {
    fn event_size(&self) -> usize {
        mem::size_of::<Self>()
            + self.input_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.output_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.output_compressed_accounts.len() * mem::size_of::<CompressedAccount>()
            + self.output_state_merkle_tree_account_indices.len()
            + self.output_leaf_indices.len() * mem::size_of::<u32>()
            + self.pubkey_array.len() * mem::size_of::<Pubkey>()
            + self
                .message
                .as_ref()
                .map(|message| message.len())
                .unwrap_or(0)
    }
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
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);

        writer.write_all(
            &(self.output_state_merkle_tree_account_indices.len() as u32).to_le_bytes(),
        )?;

        for index in self.output_state_merkle_tree_account_indices.iter() {
            writer.write_all(&[*index])?;
        }

        writer.write_all(&(self.output_leaf_indices.len() as u32).to_le_bytes())?;
        for index in self.output_leaf_indices.iter() {
            writer.write_all(&index.to_le_bytes())?;
        }

        match self.relay_fee {
            Some(relay_fee) => {
                writer.write_all(&[1])?;
                writer.write_all(&relay_fee.to_le_bytes())
            }
            None => writer.write_all(&[0]),
        }?;

        writer.write_all(&[self.is_compress as u8])?;

        match self.compression_lamports {
            Some(compression_lamports) => {
                writer.write_all(&[1])?;
                writer.write_all(&compression_lamports.to_le_bytes())
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

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info>(
    inputs: InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
) -> Result<()> {
    // TODO: add message and compression_lamports
    let event = PublicTransactionEvent {
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_compressed_accounts: inputs.output_compressed_accounts,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
        output_leaf_indices,
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compression_lamports: None,
        message: None,
        is_compress: false,
    };

    if ctx.accounts.noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(crate::ErrorCode::InvalidNoopPubkey);
    }
    let mut data = Vec::with_capacity(event.event_size());
    // TODO: add compression lamports
    event.man_serialize(&mut data)?;
    let instruction = Instruction {
        program_id: ctx.accounts.noop_program.key(),
        accounts: vec![],
        data,
    };
    invoke(&instruction, &[ctx.accounts.noop_program.to_account_info()])?;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::compressed_account::CompressedAccountData;
    use rand::Rng;

    #[test]
    fn test_manual_vs_borsh_serialization() {
        // Create a sample `PublicTransactionEvent` instance
        let event = PublicTransactionEvent {
            input_compressed_account_hashes: vec![[0u8; 32], [1u8; 32]],
            output_compressed_account_hashes: vec![[2u8; 32], [3u8; 32]],
            output_compressed_accounts: vec![CompressedAccount {
                owner: Pubkey::new_unique(),
                lamports: 100,
                address: Some([5u8; 32]),
                data: Some(CompressedAccountData {
                    discriminator: [6u8; 8],
                    data: vec![7u8; 32],
                    data_hash: [8u8; 32],
                }),
            }],
            output_state_merkle_tree_account_indices: vec![1, 2, 3],
            output_leaf_indices: vec![4, 5, 6],
            relay_fee: Some(1000),
            is_compress: true,
            compression_lamports: Some(5000),
            pubkey_array: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            message: Some(vec![8, 9, 10]),
        };

        // Serialize using Borsh
        let borsh_serialized = event.try_to_vec().unwrap();

        // Serialize manually
        let mut manual_serialized = Vec::new();
        event
            .man_serialize(
                &mut manual_serialized,
                // &event.input_compressed_account_hashes,
                // &event.output_compressed_account_hashes,
                // &event.output_compressed_accounts,
                // &event.output_state_merkle_tree_account_indices,
                // &event.output_leaf_indices,
                // &event.relay_fee,
                // event.is_compress,
                // &event.compression_lamports,
                // &event.pubkey_array,
                // &event.message,
            )
            .unwrap();

        // Compare the two byte arrays
        assert_eq!(
            borsh_serialized, manual_serialized,
            "Borsh and manual serialization results should match"
        );
    }

    #[test]
    fn test_serialization_consistency() {
        let mut rng = rand::thread_rng();

        for _ in 0..1_000_000 {
            let input_hashes: Vec<[u8; 32]> =
                (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let output_hashes: Vec<[u8; 32]> =
                (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let output_accounts: Vec<CompressedAccount> = (0..rng.gen_range(1..10))
                .map(|_| CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: rng.gen(),
                    address: Some(rng.gen()),
                    data: None,
                })
                .collect();
            let merkle_indices: Vec<u8> = (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let leaf_indices: Vec<u32> = (0..rng.gen_range(1..10)).map(|_| rng.gen()).collect();
            let pubkeys: Vec<Pubkey> = (0..rng.gen_range(1..10))
                .map(|_| Pubkey::new_unique())
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
                output_state_merkle_tree_account_indices: merkle_indices,
                output_leaf_indices: leaf_indices,
                relay_fee: if rng.gen() { Some(rng.gen()) } else { None },
                is_compress: rng.gen(),
                compression_lamports: if rng.gen() { Some(rng.gen()) } else { None },
                pubkey_array: pubkeys,
                message,
            };

            let borsh_serialized = event.try_to_vec().unwrap();
            let mut manual_serialized = Vec::new();
            event
                .man_serialize(
                    &mut manual_serialized,
                    // &event.input_compressed_account_hashes,
                    // &event.output_compressed_account_hashes,
                    // &event.output_compressed_accounts,
                    // &event.output_state_merkle_tree_account_indices,
                    // &event.output_leaf_indices,
                    // &event.relay_fee,
                    // event.is_compress,
                    // &event.compression_lamports,
                    // &event.pubkey_array,
                    // &event.message,
                )
                .unwrap();

            assert_eq!(
                borsh_serialized, manual_serialized,
                "Borsh and manual serialization results should match"
            );
        }
    }
}
