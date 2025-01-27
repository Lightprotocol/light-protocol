use anchor_lang::Discriminator;
use light_utils::instruction::{
    event::{MerkleTreeSequenceNumber, PublicTransactionEvent},
    insert_into_queues::AppendNullifyCreateAddressInputsIndexer,
    instruction_data::OutputCompressedAccountWithPackedContext,
    instruction_data_zero_copy::{
        ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZInstructionDataInvokeCpiWithReadOnly,
    },
};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use solana_sdk::pubkey::Pubkey;

// TODO: remove unwraps
/// We piece the event together from 2 instructions:
/// 1. light_system_program::{Invoke, InvokeCpi, InvokeCpiReadOnly} (one of the 3)
/// 2. account_compression::InsertIntoQueues
///
/// Steps:
/// 1. search instruction which matches one of the system instructions
/// 2. search instruction which matches InsertIntoQueues
/// 3. Populate pubkey array with remaining accounts.
pub fn event_from_light_transaction(
    instructions: &[&[u8]],
    remaining_accounts: Vec<Pubkey>,
) -> Result<Option<PublicTransactionEvent>, ZeroCopyError> {
    let event = instructions
        .iter()
        .find_map(|x| match_system_program_instruction(x).unwrap());
    if let Some(mut event) = event {
        let res = instructions
            .iter()
            .any(|x| match_account_compression_program_instruction(x, &mut event).unwrap());
        if res {
            event.pubkey_array = remaining_accounts;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn match_account_compression_program_instruction(
    instruction: &[u8],
    event: &mut PublicTransactionEvent,
) -> Result<bool, ZeroCopyError> {
    if instruction.len() < 8 {
        return Ok(false);
    }
    let instruction_discriminator = instruction[0..8].try_into().unwrap();
    match instruction_discriminator {
        account_compression::instruction::InsertIntoQueues::DISCRIMINATOR => {
            let (data, _) =
                AppendNullifyCreateAddressInputsIndexer::zero_copy_at(&instruction[8..])?;
            event.input_compressed_account_hashes =
                data.nullifiers.iter().map(|x| x.account_hash).collect();
            event.output_compressed_account_hashes = data.leaves.iter().map(|x| x.leaf).collect();
            event.sequence_numbers = data
                .sequence_numbers
                .iter()
                .map(|x| MerkleTreeSequenceNumber {
                    pubkey: x.pubkey.to_bytes().into(),
                    seq: x.seq.into(),
                })
                .collect();
            event.output_leaf_indices = data
                .output_leaf_indices
                .iter()
                .map(|x| (*x).into())
                .collect();
            Ok(true)
        }
        _ => Ok(false),
    }
}

// impl<'a> From<&ZOutputCompressedAccountWithPackedContext<'a>>
//     for OutputCompressedAccountWithPackedContext
// {
//     fn from(output_compressed_account: &ZOutputCompressedAccountWithPackedContext<'a>) -> Self {
//         OutputCompressedAccountWithPackedContext {
//             compressed_account: (&output_compressed_account.compressed_account).into(),
//             merkle_tree_index: output_compressed_account.merkle_tree_index,
//         }
//     }
// }

// impl From<ZCompressedAccountData<'_>> for CompressedAccountData {
//     fn from(compressed_account_data: ZCompressedAccountData) -> Self {
//         CompressedAccountData {
//             discriminator: *compressed_account_data.discriminator,
//             data: compressed_account_data.data.to_vec(),
//             data_hash: *compressed_account_data.data_hash,
//         }
//     }
// }

// impl From<&ZCompressedAccount<'_>> for CompressedAccount {
//     fn from(compressed_account: &ZCompressedAccount) -> Self {
//         let data = compressed_account
//             .data
//             .as_ref()
//             .map(CompressedAccountData::from);
//         CompressedAccount {
//             owner: compressed_account.owner.into(),
//             lamports: compressed_account.lamports.into(),
//             address: compressed_account.address.map(|x| *x),
//             data,
//         }
//     }
// }

pub fn match_system_program_instruction(
    instruction: &[u8],
) -> Result<Option<PublicTransactionEvent>, ZeroCopyError> {
    if instruction.len() < 8 {
        return Ok(None);
    }
    let mut event = PublicTransactionEvent::default();
    let instruction_discriminator = instruction[0..8].try_into().unwrap();
    match instruction_discriminator {
        light_system_program::instruction::Invoke::DISCRIMINATOR => {
            let (data, _) = ZInstructionDataInvoke::zero_copy_at(&instruction[8..])?;
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            // event.message = data.message;
            Ok(Some(event))
        }
        light_system_program::instruction::InvokeCpi::DISCRIMINATOR => {
            let (data, _) = ZInstructionDataInvokeCpi::zero_copy_at(&instruction[8..])?;
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect::<Vec<_>>();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            // event.message = data.message;
            Ok(Some(event))
        }
        light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR => {
            let (data, _) = ZInstructionDataInvokeCpiWithReadOnly::zero_copy_at(&instruction[8..])?;
            let data = data.invoke_cpi;
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect::<Vec<_>>();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            // event.message = data.message;
            Ok(Some(event))
        }
        _ => {
            return Ok(None);
        }
    }
}
