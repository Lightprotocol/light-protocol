#![cfg(not(target_os = "solana"))]
use std::collections::HashMap;

use account_compression::{AccountMeta, Pubkey};
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use solana_sdk::instruction::Instruction;

use crate::{
    utxo::{OutUtxo, SerializedUtxos, Utxo},
    InstructionDataTransfer, InstructionDataTransfer2,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedProof {
    pub proof_a: [u8; 32],
    pub proof_b: [u8; 64],
    pub proof_c: [u8; 32],
}
#[allow(clippy::too_many_arguments)]
pub fn create_execute_compressed_instruction(
    payer: &Pubkey,
    in_utxos: &[Utxo],
    out_utxos: &[OutUtxo],
    in_utxo_merkle_tree_pubkeys: &[Pubkey],
    nullifier_array_pubkeys: &[Pubkey],
    out_utxo_merkle_tree_pubkeys: &[Pubkey],
    root_indices: &[u16],
    proof: &CompressedProof,
) -> Instruction {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let mut _in_utxos = Vec::<(Utxo, u8, u8)>::new();
    for (i, mt) in in_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i);
            }
        };
        _in_utxos.push((
            in_utxos[i].clone(),
            *remaining_accounts.get(mt).unwrap() as u8,
            0,
        ));
    }
    let len: usize = remaining_accounts.len();
    for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        _in_utxos[i].2 = *remaining_accounts.get(mt).unwrap() as u8;
    }
    let len: usize = remaining_accounts.len();
    let mut _out_utxos = Vec::<(OutUtxo, u8)>::new();

    for (i, mt) in out_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        _out_utxos.push((
            out_utxos[i].clone(),
            *remaining_accounts.get(mt).unwrap() as u8,
        ));
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
        low_element_indices: vec![0u16; in_utxos.len()],
        rpc_fee: None,
        in_utxos: _in_utxos,
        out_utxos: _out_utxos,
        root_indices: root_indices.to_vec(),
        proof_a: proof.proof_a,
        proof_b: proof.proof_b,
        proof_c: proof.proof_c,
    };

    let mut inputs = Vec::new();
    // inputs_struct.serialize(&mut inputs).unwrap();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();
    let instruction_data = crate::instruction::ExecuteCompressedTransaction { inputs };
    // InstructionDataTransfer::deserialize(&mut inputs.as_slice()).unwrap();
    let accounts = crate::accounts::TransferInstruction {
        signer: *payer,
        // authority_pda: get_cpi_authority_pda(&crate::ID),
        registered_program_pda: get_registered_program_pda(&crate::ID),
        noop_program: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        account_compression_program: account_compression::ID,
        psp_account_compression_authority: get_cpi_authority_pda(&crate::ID),
        cpi_signature_account: None,
    };
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

// TODO: make more efficient
#[allow(clippy::too_many_arguments)]
pub fn create_execute_compressed_opt_instruction(
    payer: &Pubkey,
    in_utxos: &[Utxo],
    out_utxos: &[OutUtxo],
    in_utxo_merkle_tree_pubkeys: &[Pubkey],
    nullifier_array_pubkeys: &[Pubkey],
    out_utxo_merkle_tree_pubkeys: &[Pubkey],
    leaf_indices: &[u32],
    root_indices: &[u16],
    proof: &CompressedProof,
) -> Instruction {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    for (i, mt) in in_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i);
            }
        };
    }
    let len: usize = remaining_accounts.len();
    /// CHECK: this depends on nulif never matching any of the statetrees 
    for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
    }
    let len: usize = remaining_accounts.len();
    for (i, mt) in out_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
    }

    let mut inputs = Vec::new();

    let accounts = crate::accounts::TransferInstruction {
        signer: *payer,
        registered_program_pda: get_registered_program_pda(&crate::ID),
        noop_program: account_compression::state::change_log_event::NOOP_PROGRAM_ID,
        account_compression_program: account_compression::ID,
        psp_account_compression_authority: get_cpi_authority_pda(&crate::ID),
        cpi_signature_account: None,
    };
    let mut utxos = SerializedUtxos {
        pubkey_array: vec![],
        u64_array: vec![],
        in_utxos: vec![],
        out_utxos: vec![],
    };
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
    let remaining_accounts_pubkeys = remaining_accounts
        .iter()
        .map(|k| k.pubkey)
        .collect::<Vec<Pubkey>>();
    let account_vec = accounts
        .to_account_metas(None)
        .iter()
        .map(|k| k.pubkey)
        .collect::<Vec<Pubkey>>();
    let all_accounts = [account_vec, remaining_accounts_pubkeys.clone()].concat();
    utxos
        .add_in_utxos(
            in_utxos,
            all_accounts.as_slice(),
            leaf_indices,
            in_utxo_merkle_tree_pubkeys,
            nullifier_array_pubkeys,
        )
        .unwrap();
    utxos
        .add_out_utxos(
            out_utxos,
            all_accounts.as_slice(),
            remaining_accounts_pubkeys.as_slice(),
            out_utxo_merkle_tree_pubkeys,
        )
        .unwrap();

    let inputs_struct = InstructionDataTransfer2 {
        low_element_indices: vec![0u16; in_utxos.len()],
        rpc_fee: None,
        root_indices: root_indices.to_vec(),
        utxos,
        proof_a: proof.proof_a,
        proof_b: proof.proof_b,
        proof_c: proof.proof_c,
    };
    InstructionDataTransfer2::serialize(&inputs_struct, &mut inputs).unwrap();
    let instruction_data = crate::instruction::ExecuteCompressedTransaction2 { inputs };
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"cpi_authority",
            account_compression::ID.to_bytes().as_slice(),
        ],
        program_id,
    )
    .0
}

#[test]
fn test_create_execute_compressed_transaction() {
    let payer = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let in_utxos = vec![
        Utxo {
            lamports: 100,
            owner: payer,
            blinding: [0u8; 32],
            data: None,
        },
        Utxo {
            lamports: 100,
            owner: payer,
            blinding: [0u8; 32],
            data: None,
        },
    ];
    let out_utxos = vec![
        OutUtxo {
            lamports: 50,
            owner: payer,
            data: None,
        },
        OutUtxo {
            lamports: 150,
            owner: recipient,
            data: None,
        },
    ];
    let merkle_tree_pubkey = Pubkey::new_unique();
    let nullifier_array_pubkey = Pubkey::new_unique();
    let in_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey, merkle_tree_pubkey];
    let nullifier_array_pubkeys = vec![nullifier_array_pubkey, nullifier_array_pubkey];
    let out_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey, merkle_tree_pubkey];
    let root_indices = vec![0, 1];
    let proof = CompressedProof {
        proof_a: [0u8; 32],
        proof_b: [0u8; 64],
        proof_c: [0u8; 32],
    };
    let instruction = create_execute_compressed_instruction(
        &payer,
        &in_utxos.clone(),
        &out_utxos.clone(),
        &in_utxo_merkle_tree_pubkeys,
        &nullifier_array_pubkeys,
        &out_utxo_merkle_tree_pubkeys,
        &root_indices.clone(),
        &proof.clone(),
    );
    assert_eq!(instruction.program_id, crate::ID);

    use account_compression::AccountDeserialize;

    let deserialized_instruction_data: InstructionDataTransfer =
        InstructionDataTransfer::try_deserialize_unchecked(
            &mut [vec![0u8; 8], instruction.data[12..].to_vec()]
                .concat()
                .as_slice(),
        )
        .unwrap();
    deserialized_instruction_data
        .in_utxos
        .iter()
        .enumerate()
        .for_each(|(i, utxo)| {
            assert_eq!(in_utxos[i], utxo.0);
        });
    deserialized_instruction_data
        .out_utxos
        .iter()
        .enumerate()
        .for_each(|(i, utxo)| {
            assert_eq!(out_utxos[i], utxo.0);
        });
    assert_eq!(deserialized_instruction_data.in_utxos.len(), 2);
    assert_eq!(deserialized_instruction_data.out_utxos.len(), 2);
    assert_eq!(deserialized_instruction_data.root_indices, root_indices);
    assert_eq!(deserialized_instruction_data.proof_a, proof.proof_a);
    assert_eq!(deserialized_instruction_data.proof_b, proof.proof_b);
    assert_eq!(deserialized_instruction_data.proof_c, proof.proof_c);
    assert_eq!(instruction.accounts[0], AccountMeta::new(payer, true));
    assert_eq!(deserialized_instruction_data.in_utxos[0].2, 1);
    assert_eq!(deserialized_instruction_data.in_utxos[1].2, 1);
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.in_utxos[0].1 as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.in_utxos[1].1 as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.in_utxos[0].2 as usize],
        AccountMeta::new(nullifier_array_pubkey, false)
    );
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.in_utxos[1].2 as usize],
        AccountMeta::new(nullifier_array_pubkey, false)
    );
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.out_utxos[0].1 as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts[6 + deserialized_instruction_data.out_utxos[1].1 as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
}
