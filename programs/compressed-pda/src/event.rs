use std::{mem, str::FromStr};

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    InstructionDataTransfer, TransferInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub input_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    // index of Merkle tree account in remaining accounts
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub output_leaf_indices: Vec<u32>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compression_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

impl PublicTransactionEvent {
    pub fn light_serialize<W: Write>(writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&(self.input_compressed_account_hashes.len() as u32).to_le_bytes())?;
        for hash in self.input_compressed_account_hashes {
            writer.write_all(hash)?;
        }

        writer.write_all(&(self.output_compressed_account_hashes.len() as u32).to_le_bytes())?;
        for hash in self.output_compressed_account_hashes {
            writer.write_all(hash)?;
        }

        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap("before output compressed accounts");
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
        writer.write_all(&(self.input_compressed_accounts.len() as u32).to_le_bytes())?;
        for i in 0..self.input_compressed_accounts.len() {
            let account = input_compressed_accounts[i].clone();
            account.serialize(writer)?;
        }
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);

        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap("before output compressed accounts");
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
        writer.write_all(&(self.output_compressed_accounts.len() as u32).to_le_bytes())?;
        for i in 0..self.output_compressed_accounts.len() {
            let account = output_compressed_accounts[i].clone();
            account.serialize(writer)?;
        }
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);

        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap("before output compressed accounts");
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
        writer.write_all(&(output_compressed_accounts.len() as u32).to_le_bytes())?;
        for i in 0..output_compressed_accounts.len() {
            let account = output_compressed_accounts[i].clone();
            account.serialize(writer)?;
        }
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);

        writer.write_all(&(output_state_merkle_tree_account_indices.len() as u32).to_le_bytes())?;
        for index in output_state_merkle_tree_account_indices {
            writer.write_all(&[*index])?;
        }

        writer.write_all(&(output_leaf_indices.len() as u32).to_le_bytes())?;
        for index in output_leaf_indices {
            writer.write_all(&index.to_le_bytes())?;
        }

        match relay_fee {
            Some(relay_fee) => {
                writer.write_all(&[1])?;
                writer.write_all(&relay_fee.to_le_bytes())
            }
            None => writer.write_all(&[0]),
        }?;

        writer.write_all(&[is_compress as u8])?;

        match compression_lamports {
            Some(compression_lamports) => {
                writer.write_all(&[1])?;
                writer.write_all(&compression_lamports.to_le_bytes())
            }
            None => writer.write_all(&[0]),
        }?;

        writer.write_all(&(pubkey_array.len() as u32).to_le_bytes())?;
        for pubkey in pubkey_array {
            writer.write_all(&pubkey.to_bytes())?;
        }

        match &message {
            Some(message) => {
                writer.write_all(&[1])?;
                writer.write_all(&(message.len() as u32).to_le_bytes())?;
                writer.write_all(message)
            }
            None => writer.write_all(&[0]),
        }?;

        Ok(())
    }

    pub fn light_try_to_vec(&self) -> Result<Vec<u8>> {
        let capacity = input_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.output_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.input_compressed_accounts * mem::size_of::<CompressedAccountWithMerkleContext>()
            + self.output_compressed_accounts * mem::size_of::<CompressedAccount>()
            + self.output_state_merkle_tree_account_indices.len() * mem::size_of::<u8>()
            + self.output_leaf_indices.len() * mem::size_of::<u32>()
            + self.pubkey_array.len() * mem::size_of::<Pubkey>()
            + self.message.map(|message| message.len()).unwrap_or(0);
        let mut vec = Vec::with_capacity(capacity);
        self.serialize(&mut vec)?;
        Ok(vec)
    }
}

#[inline(never)]
pub fn invoke_indexer_transaction_event<T>(event: &T, noop_program: &AccountInfo) -> Result<()>
where
    T: AnchorSerialize,
{
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(crate::ErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data: event.light_to_vec()?,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    input_compressed_account_hashes: &[[u8; 32]],
    output_compressed_account_hashes: &[[u8; 32]],
    output_leaf_indices: &[u32],
) -> Result<()> {
    // TODO: add message and compression_lamports
    let event = PublicTransactionEvent {
        input_compressed_account_hashes: input_compressed_account_hashes.to_vec(),
        output_compressed_account_hashes: output_compressed_account_hashes.to_vec(),
        input_compressed_accounts: inputs.input_compressed_accounts_with_merkle_context.clone(),
        input_compressed_accounts: inputs.output_compressed_accounts.to_vec(),
        output_state_merkle_tree_account_indices: inputs
            .output_state_merkle_tree_account_indices
            .to_vec(),
        output_leaf_indices: output_leaf_indices.to_vec(),
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compression_lamports: None,
        message: None,
        is_compress: false,
    };
    invoke_indexer_transaction_event(&event, &ctx.accounts.noop_program)?;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use rand::{
        distributions::{Distribution, Standard},
        Rng,
    };

    #[test]
    fn test_manual_vs_borsh_serialization() {
        // Create a sample `PublicTransactionEvent` instance
        let event = PublicTransactionEvent {
            input_compressed_account_hashes: vec![[0u8; 32], [1u8; 32]],
            output_compressed_account_hashes: vec![[2u8; 32], [3u8; 32]],
            input_compressed_accounts: vec![CompressedAccount {
                owner: Pubkey::new_unique(),
                lamports: 100,
                address: Some([5u8; 32]),
                data: Some(CompressedAccountData {
                    discriminator: [6u8; 8],
                    data: vec![7u8; 32],
                    data_hash: [8u8; 32],
                }),
            }],
            input_compressed_accounts: vec![CompressedAccount {
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

        // Serialize with our method
        let light_serialized = event.light_try_to_vec().unwrap();

        // Compare the two byte arrays
        assert_eq!(
            borsh_serialized, light_serialized,
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
            let input_accounts: Vec<CompressedAccount> = (0..rng.gen_range(1..10))
                .map(|_| CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: rng.gen(),
                    address: Some(rng.gen()),
                    data: None,
                })
                .collect();
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
                input_compressed_accounts: input_accounts,
                input_compressed_accounts: output_accounts,
                output_state_merkle_tree_account_indices: merkle_indices,
                output_leaf_indices: leaf_indices,
                relay_fee: if rng.gen() { Some(rng.gen()) } else { None },
                is_compress: rng.gen(),
                compression_lamports: if rng.gen() { Some(rng.gen()) } else { None },
                pubkey_array: pubkeys,
                message,
            };

            let borsh_serialized = event.try_to_vec().unwrap();
            let light_serialized = event.light_try_to_vec().unwrap();

            assert_eq!(
                borsh_serialized, light_serialized,
                "Borsh and manual serialization results should match"
            );
        }
    }
}
