#![cfg(not(target_os = "solana"))]
use std::collections::HashMap;

use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use super::compressed_account::{
    CompressedAccount, MerkleContext, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
};
use crate::{
    invoke::{processor::CompressedProof, sol_compression::SOL_POOL_PDA_SEED},
    utils::{get_cpi_authority_pda, get_registered_program_pda},
    InstructionDataInvoke, NewAddressParams, NewAddressParamsPacked,
    OutputCompressedAccountWithPackedContext,
};

pub fn get_sol_pool_pda() -> Pubkey {
    Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &crate::ID).0
}

#[allow(clippy::too_many_arguments)]
pub fn create_invoke_instruction(
    fee_payer: &Pubkey,
    payer: &Pubkey,
    input_compressed_accounts: &[CompressedAccount],
    output_compressed_accounts: &[CompressedAccount],
    merkle_context: &[MerkleContext],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    input_root_indices: &[u16],
    new_address_params: &[NewAddressParams],
    proof: Option<CompressedProof>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    decompression_recipient: Option<Pubkey>,
    sort: bool,
) -> Instruction {
    let (remaining_accounts, mut inputs_struct) =
        create_invoke_instruction_data_and_remaining_accounts(
            new_address_params,
            merkle_context,
            input_compressed_accounts,
            input_root_indices,
            output_compressed_account_merkle_tree_pubkeys,
            output_compressed_accounts,
            proof,
            compress_or_decompress_lamports,
            is_compress,
        );
    if sort {
        inputs_struct
            .output_compressed_accounts
            .sort_by(|a, b| a.merkle_tree_index.cmp(&b.merkle_tree_index));
    }
    let mut inputs = Vec::new();

    InstructionDataInvoke::serialize(&inputs_struct, &mut inputs).unwrap();

    let instruction_data = crate::instruction::Invoke { inputs };

    let sol_pool_pda = compress_or_decompress_lamports.map(|_| get_sol_pool_pda());

    let accounts = crate::accounts::InvokeInstruction {
        fee_payer: *fee_payer,
        authority: *payer,
        registered_program_pda: get_registered_program_pda(&crate::ID),
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        account_compression_program: account_compression::ID,
        account_compression_authority: get_cpi_authority_pda(&crate::ID),
        sol_pool_pda,
        decompression_recipient,
        system_program: solana_sdk::system_program::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_invoke_instruction_data_and_remaining_accounts(
    new_address_params: &[NewAddressParams],
    merkle_context: &[MerkleContext],
    input_compressed_accounts: &[CompressedAccount],
    input_root_indices: &[u16],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    output_compressed_accounts: &[CompressedAccount],
    proof: Option<CompressedProof>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
) -> (Vec<AccountMeta>, InstructionDataInvoke) {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let mut _input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext> =
        Vec::<PackedCompressedAccountWithMerkleContext>::new();
    let mut index = 0;
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(|x| NewAddressParamsPacked {
            seed: x.seed,
            address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            address_merkle_tree_account_index: 0, // will be assigned later
            address_queue_account_index: 0,       // will be assigned later
        })
        .collect::<Vec<NewAddressParamsPacked>>();
    for (i, context) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&context.merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(context.merkle_tree_pubkey, index);
                index += 1;
            }
        };
        _input_compressed_accounts.push(PackedCompressedAccountWithMerkleContext {
            compressed_account: input_compressed_accounts[i].clone(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: *remaining_accounts
                    .get(&context.merkle_tree_pubkey)
                    .unwrap() as u8,
                nullifier_queue_pubkey_index: 0,
                leaf_index: context.leaf_index,
                queue_index: None,
            },
            root_index: input_root_indices[i],
        });
    }

    for (i, context) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&context.nullifier_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(context.nullifier_queue_pubkey, index);
                index += 1;
            }
        };
        _input_compressed_accounts[i]
            .merkle_context
            .nullifier_queue_pubkey_index = *remaining_accounts
            .get(&context.nullifier_queue_pubkey)
            .unwrap() as u8;
    }

    let mut output_compressed_accounts_with_context: Vec<OutputCompressedAccountWithPackedContext> =
        Vec::<OutputCompressedAccountWithPackedContext>::new();

    for (i, mt) in output_compressed_account_merkle_tree_pubkeys
        .iter()
        .enumerate()
    {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, index);
                index += 1;
            }
        };

        output_compressed_accounts_with_context.push(OutputCompressedAccountWithPackedContext {
            compressed_account: output_compressed_accounts[i].clone(),
            merkle_tree_index: *remaining_accounts.get(mt).unwrap() as u8,
        });
    }

    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_merkle_tree_pubkey, index);
                index += 1;
            }
        };
        new_address_params_packed[i].address_merkle_tree_account_index = *remaining_accounts
            .get(&params.address_merkle_tree_pubkey)
            .unwrap()
            as u8;
    }

    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_queue_pubkey, index);
                index += 1;
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

    let inputs_struct = InstructionDataInvoke {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: _input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts_with_context,
        proof,
        new_address_params: new_address_params_packed,
        compress_or_decompress_lamports,
        is_compress,
    };
    (remaining_accounts, inputs_struct)
}

#[cfg(test)]
mod test {
    use anchor_lang::AnchorDeserialize;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    #[test]
    fn test_create_execute_compressed_transaction() {
        let payer = Keypair::new().pubkey();
        let recipient = Keypair::new().pubkey();
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
        let merkle_tree_indices = vec![0, 2];
        let merkle_tree_pubkey = Keypair::new().pubkey();
        let merkle_tree_pubkey_1 = Keypair::new().pubkey();

        let nullifier_array_pubkey = Keypair::new().pubkey();
        let input_merkle_context = vec![
            MerkleContext {
                merkle_tree_pubkey,
                nullifier_queue_pubkey: nullifier_array_pubkey,
                leaf_index: 0,
                queue_index: None,
            },
            MerkleContext {
                merkle_tree_pubkey,
                nullifier_queue_pubkey: nullifier_array_pubkey,
                leaf_index: 1,
                queue_index: None,
            },
        ];

        let output_compressed_account_merkle_tree_pubkeys =
            vec![merkle_tree_pubkey, merkle_tree_pubkey_1];
        let input_root_indices = vec![0, 1];
        let proof = CompressedProof {
            a: [0u8; 32],
            b: [1u8; 64],
            c: [0u8; 32],
        };
        let instruction = create_invoke_instruction(
            &payer,
            &payer,
            &input_compressed_accounts.clone(),
            &output_compressed_accounts.clone(),
            &input_merkle_context,
            &output_compressed_account_merkle_tree_pubkeys,
            &input_root_indices.clone(),
            Vec::<NewAddressParams>::new().as_slice(),
            Some(proof.clone()),
            Some(100),
            true,
            None,
            true,
        );
        assert_eq!(instruction.program_id, crate::ID);

        let deserialized_instruction_data: InstructionDataInvoke =
            InstructionDataInvoke::deserialize(&mut instruction.data[12..].as_ref()).unwrap();
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
                assert_eq!(
                    OutputCompressedAccountWithPackedContext {
                        compressed_account: output_compressed_accounts[i].clone(),
                        merkle_tree_index: merkle_tree_indices[i] as u8
                    },
                    *compressed_account
                );
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
            deserialized_instruction_data
                .compress_or_decompress_lamports
                .unwrap(),
            100
        );
        assert_eq!(deserialized_instruction_data.is_compress, true);
        let ref_account_meta = AccountMeta::new(payer, true);
        assert_eq!(instruction.accounts[0], ref_account_meta);
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .nullifier_queue_pubkey_index,
            1
        );
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .nullifier_queue_pubkey_index,
            1
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .nullifier_queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .nullifier_queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[0]
                .merkle_tree_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[1]
                .merkle_tree_index as usize],
            AccountMeta::new(merkle_tree_pubkey_1, false)
        );
    }
}
