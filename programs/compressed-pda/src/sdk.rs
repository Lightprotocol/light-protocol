#![cfg(not(target_os = "solana"))]
use std::collections::HashMap;

use account_compression::{AccountMeta, Pubkey};
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use solana_sdk::instruction::Instruction;

use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    utils::{get_cpi_authority_pda, get_registered_program_pda, CompressedProof},
    InstructionDataTransfer, NewAddressParamsPacked, COMPRESSED_SOL_PDA_SEED,
};

pub fn get_compressed_sol_pda() -> Pubkey {
    Pubkey::find_program_address(&[COMPRESSED_SOL_PDA_SEED], &crate::ID).0
}

#[allow(clippy::too_many_arguments)]
pub fn create_execute_compressed_instruction(
    payer: &Pubkey,
    input_compressed_accounts: &[CompressedAccount],
    output_compressed_accounts: &[CompressedAccount],
    input_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    input_compressed_accounts_leaf_indices: &[u32],
    nullifier_queue_pubkeys: &[Pubkey],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    input_root_indices: &[u16],
    new_address_params: &[crate::NewAddressParams],
    proof: &CompressedProof,
    compression_lamports: Option<u64>,
    is_compress: bool,
    compression_recipient: Option<Pubkey>,
) -> Instruction {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let mut _input_compressed_accounts: Vec<CompressedAccountWithMerkleContext> =
        Vec::<CompressedAccountWithMerkleContext>::new();
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(|x| NewAddressParamsPacked {
            seed: x.seed,
            address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            address_merkle_tree_account_index: 0, // will be assigned later
            address_queue_account_index: 0,       // will be assigned later
        })
        .collect::<Vec<NewAddressParamsPacked>>();
    for (i, mt) in input_compressed_account_merkle_tree_pubkeys
        .iter()
        .enumerate()
    {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i);
            }
        };
        _input_compressed_accounts.push(CompressedAccountWithMerkleContext {
            compressed_account: input_compressed_accounts[i].clone(),
            merkle_tree_pubkey_index: *remaining_accounts.get(mt).unwrap() as u8,
            nullifier_queue_pubkey_index: 0,
            leaf_index: input_compressed_accounts_leaf_indices[i],
        });
    }
    let len: usize = remaining_accounts.len();
    for (i, mt) in nullifier_queue_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        _input_compressed_accounts[i].nullifier_queue_pubkey_index =
            *remaining_accounts.get(mt).unwrap() as u8;
    }
    let len: usize = remaining_accounts.len();
    let mut output_state_merkle_tree_account_indices: Vec<u8> = Vec::<u8>::new();

    for (i, mt) in output_compressed_account_merkle_tree_pubkeys
        .iter()
        .enumerate()
    {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        output_state_merkle_tree_account_indices.push(*remaining_accounts.get(mt).unwrap() as u8);
    }
    let len: usize = remaining_accounts.len();
    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_merkle_tree_pubkey, i + len);
            }
        };
        new_address_params_packed[i].address_merkle_tree_account_index = *remaining_accounts
            .get(&params.address_merkle_tree_pubkey)
            .unwrap()
            as u8;
    }

    let len: usize = remaining_accounts.len();
    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_queue_pubkey, i + len);
            }
        };
        new_address_params_packed[i].address_queue_account_index = *remaining_accounts
            .get(&params.address_queue_pubkey)
            .unwrap() as u8;
    }
    let mut remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, i)| (AccountMeta::new(*k, false), *i))
        .collect::<Vec<(AccountMeta, usize)>>();
    // hash maps are not sorted so we need to sort manually and collect into a vector again
    remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
    let remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<AccountMeta>>();

    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: _input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts.to_vec(),
        output_state_merkle_tree_account_indices,
        input_root_indices: input_root_indices.to_vec(),
        proof: Some(proof.clone()),
        new_address_params: new_address_params_packed,
        compression_lamports,
        is_compress,
    };

    let mut inputs = Vec::new();

    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let instruction_data = crate::instruction::ExecuteCompressedTransaction { inputs };

    let compressed_sol_pda = compression_lamports.map(|_| get_compressed_sol_pda());

    let accounts = crate::accounts::TransferInstruction {
        signer: *payer,
        // authority_pda: get_cpi_authority_pda(&crate::ID),
        registered_program_pda: get_registered_program_pda(&crate::ID),
        noop_program: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        account_compression_program: account_compression::ID,
        account_compression_authority: get_cpi_authority_pda(&crate::ID),
        cpi_signature_account: None,
        invoking_program: None,
        compressed_sol_pda,
        compression_recipient,
        system_program: Some(solana_sdk::system_program::ID),
    };
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

// // TODO: make more efficient
// #[allow(clippy::too_many_arguments)]
// pub fn create_execute_compressed_opt_instruction(
//     payer: &Pubkey,
//     input_compressed_accounts: &[CompressedAccount],
//     output_compressed_accounts: &[OutUtxo],
//     input_compressed_account_merkle_tree_pubkeys: &[Pubkey],
//     nullifier_queue_pubkeys: &[Pubkey],
//     output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
//     leaf_indices: &[u32],
//     input_root_indices: &[u16],
//     proof: &CompressedProof,
// ) -> Instruction {
//     let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
//     for (i, mt) in input_compressed_account_merkle_tree_pubkeys.iter().enumerate() {
//         match remaining_accounts.get(mt) {
//             Some(_) => {}
//             None => {
//                 remaining_accounts.insert(*mt, i);
//             }
//         };
//     }
//     let len: usize = remaining_accounts.len();
//     // Note: this depends on nulifier never matching any of the statetrees.
//     for (i, mt) in nullifier_queue_pubkeys.iter().enumerate() {
//         match remaining_accounts.get(mt) {
//             Some(_) => {}
//             None => {
//                 remaining_accounts.insert(*mt, i + len);
//             }
//         };
//     }
//     let len: usize = remaining_accounts.len();
//     for (i, mt) in output_compressed_account_merkle_tree_pubkeys.iter().enumerate() {
//         match remaining_accounts.get(mt) {
//             Some(_) => {}
//             None => {
//                 remaining_accounts.insert(*mt, i + len);
//             }
//         };
//     }

//     let mut inputs = Vec::new();

//     let accounts = crate::accounts::TransferInstruction {
//         signer: *payer,
//         registered_program_pda: get_registered_program_pda(&crate::ID),
//         noop_program: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
//         account_compression_program: account_compression::ID,
//         account_compression_authority: get_cpi_authority_pda(&crate::ID),
//         cpi_signature_account: None,
//         invoking_program: None,
//     };
//     let mut utxos = SerializedUtxos {
//         pubkey_array: vec![],
//         u64_array: vec![],
//         input_compressed_accounts: vec![],
//         output_compressed_accounts: vec![],
//     };
//     let mut remaining_accounts = remaining_accounts
//         .iter()
//         .map(|(k, i)| (AccountMeta::new(*k, false), *i))
//         .collect::<Vec<(AccountMeta, usize)>>();
//     // hash maps are not sorted so we need to sort manually and collect into a vector again
//     remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
//     let remaining_accounts = remaining_accounts
//         .iter()
//         .map(|(k, _)| k.clone())
//         .collect::<Vec<AccountMeta>>();
//     let remaining_accounts_pubkeys = remaining_accounts
//         .iter()
//         .map(|k| k.pubkey)
//         .collect::<Vec<Pubkey>>();
//     let account_vec = accounts
//         .to_account_metas(None)
//         .iter()
//         .map(|k| k.pubkey)
//         .collect::<Vec<Pubkey>>();
//     let all_accounts = [account_vec, remaining_accounts_pubkeys.clone()].concat();
//     utxos
//         .add_input_compressed_accounts(
//             input_compressed_accounts,
//             all_accounts.as_slice(),
//             leaf_indices,
//             input_compressed_account_merkle_tree_pubkeys,
//             nullifier_queue_pubkeys,
//         )
//         .unwrap();
//     utxos
//         .add_output_compressed_accounts(
//             output_compressed_accounts,
//             all_accounts.as_slice(),
//             remaining_accounts_pubkeys.as_slice(),
//             output_compressed_account_merkle_tree_pubkeys,
//         )
//         .unwrap();

//     let inputs_struct = InstructionDataTransfer2 {
//         low_element_indices: vec![0u16; input_compressed_accounts.len()],
//         relay_fee: None,
//         input_root_indices: input_root_indices.to_vec(),
//         utxos,
//         proof: Some(proof.clone()),
//     };
//     InstructionDataTransfer2::serialize(&inputs_struct, &mut inputs).unwrap();
//     let instruction_data = crate::instruction::ExecuteCompressedTransaction2 { inputs };
//     Instruction {
//         program_id: crate::ID,
//         accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
//         data: instruction_data.data(),
//     }
// }

#[cfg(test)]
mod test {
    use anchor_lang::AnchorDeserialize;

    use super::*;
    use crate::{utils::CompressedProof, NewAddressParams};
    #[test]
    fn test_create_execute_compressed_transaction() {
        let payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let input_compressed_accounts = vec![
            CompressedAccount {
                lamports: 100,
                owner: payer,
                address: None,
                data: None,
            },
            CompressedAccount {
                lamports: 100,
                owner: payer,
                address: None,
                data: None,
            },
        ];
        let output_compressed_accounts = vec![
            CompressedAccount {
                lamports: 50,
                owner: payer,
                address: None,
                data: None,
            },
            CompressedAccount {
                lamports: 150,
                owner: recipient,
                address: None,
                data: None,
            },
        ];
        let merkle_tree_pubkey = Pubkey::new_unique();
        let nullifier_array_pubkey = Pubkey::new_unique();
        let input_compressed_account_merkle_tree_pubkeys =
            vec![merkle_tree_pubkey, merkle_tree_pubkey];
        let nullifier_queue_pubkeys = vec![nullifier_array_pubkey, nullifier_array_pubkey];
        let output_compressed_account_merkle_tree_pubkeys =
            vec![merkle_tree_pubkey, merkle_tree_pubkey];
        let input_root_indices = vec![0, 1];
        let proof = CompressedProof {
            a: [0u8; 32],
            b: [0u8; 64],
            c: [0u8; 32],
        };
        let input_compressed_account_leaf_indices = vec![0, 1];
        let instruction = create_execute_compressed_instruction(
            &payer,
            &input_compressed_accounts.clone(),
            &output_compressed_accounts.clone(),
            &input_compressed_account_merkle_tree_pubkeys,
            &input_compressed_account_leaf_indices,
            &nullifier_queue_pubkeys,
            &output_compressed_account_merkle_tree_pubkeys,
            &input_root_indices.clone(),
            Vec::<NewAddressParams>::new().as_slice(),
            &proof.clone(),
            Some(100),
            true,
            None,
        );
        assert_eq!(instruction.program_id, crate::ID);

        let deserialized_instruction_data: InstructionDataTransfer =
            InstructionDataTransfer::deserialize(&mut instruction.data[12..].as_ref()).unwrap();
        deserialized_instruction_data
            .input_compressed_accounts_with_merkle_context
            .iter()
            .enumerate()
            .for_each(|(i, compressed_account_with_context)| {
                assert_eq!(
                    input_compressed_accounts[i],
                    compressed_account_with_context.compressed_account
                );
            });
        deserialized_instruction_data
            .output_compressed_accounts
            .iter()
            .enumerate()
            .for_each(|(i, compressed_account)| {
                assert_eq!(output_compressed_accounts[i], *compressed_account);
            });
        assert_eq!(
            deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context
                .len(),
            2
        );
        assert_eq!(
            deserialized_instruction_data
                .output_compressed_accounts
                .len(),
            2
        );
        assert_eq!(
            deserialized_instruction_data.input_root_indices,
            input_root_indices
        );
        assert_eq!(
            deserialized_instruction_data.proof.clone().unwrap().a,
            proof.a
        );
        assert_eq!(
            deserialized_instruction_data.proof.clone().unwrap().b,
            proof.b
        );
        assert_eq!(
            deserialized_instruction_data.proof.clone().unwrap().c,
            proof.c
        );
        assert_eq!(
            deserialized_instruction_data.compression_lamports.unwrap(),
            100
        );
        assert_eq!(deserialized_instruction_data.is_compress, true);
        let mut ref_account_meta = AccountMeta::new(payer, true);
        ref_account_meta.is_writable = false;
        assert_eq!(instruction.accounts[0], ref_account_meta);
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
                .nullifier_queue_pubkey_index,
            1
        );
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
                .nullifier_queue_pubkey_index,
            1
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
                    .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
                    .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
                    .nullifier_queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
                    .nullifier_queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.output_state_merkle_tree_account_indices[0]
                    as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[10
                + deserialized_instruction_data.output_state_merkle_tree_account_indices[1]
                    as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
    }
}
