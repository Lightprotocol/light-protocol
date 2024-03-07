#![cfg(not(target_os = "solana"))]
use std::collections::HashMap;

use account_compression::{AccountMeta, Pubkey};
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use solana_sdk::instruction::Instruction;

use crate::{
    utils::{get_cpi_authority_pda, get_registered_program_pda},
    utxo::{InUtxoTuple, OutUtxo, OutUtxoTuple, SerializedUtxos, Utxo},
    InstructionDataTransfer, InstructionDataTransfer2, ProofCompressed,
};

#[allow(clippy::too_many_arguments)]
pub fn create_execute_compressed_instruction(
    payer: &Pubkey,
    in_utxos: &[Utxo],
    out_utxos: &[OutUtxo],
    in_utxo_merkle_tree_pubkeys: &[Pubkey],
    nullifier_array_pubkeys: &[Pubkey],
    out_utxo_merkle_tree_pubkeys: &[Pubkey],
    root_indices: &[u16],
    proof: &ProofCompressed,
) -> Instruction {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let mut _in_utxos: Vec<InUtxoTuple> = Vec::<InUtxoTuple>::new();
    for (i, mt) in in_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i);
            }
        };
        _in_utxos.push(InUtxoTuple {
            in_utxo: in_utxos[i].clone(),
            index_mt_account: *remaining_accounts.get(mt).unwrap() as u8,
            index_nullifier_array_account: 0,
        });
    }
    let len: usize = remaining_accounts.len();
    for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        _in_utxos[i].index_nullifier_array_account = *remaining_accounts.get(mt).unwrap() as u8;
    }
    let len: usize = remaining_accounts.len();
    let mut _out_utxos: Vec<OutUtxoTuple> = Vec::<OutUtxoTuple>::new();

    for (i, mt) in out_utxo_merkle_tree_pubkeys.iter().enumerate() {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, i + len);
            }
        };
        _out_utxos.push(OutUtxoTuple {
            out_utxo: out_utxos[i].clone(),
            index_mt_account: *remaining_accounts.get(mt).unwrap() as u8,
        });
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
        relay_fee: None,
        in_utxos: _in_utxos,
        out_utxos: _out_utxos,
        root_indices: root_indices.to_vec(),
        proof: Some(proof.clone()),
    };

    println!("inputs_struct {:?}", inputs_struct);
    let mut inputs = Vec::new();
    // inputs_struct.serialize(&mut inputs).unwrap();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    println!("encoded inputs {:?}", inputs);
    println!("encoded inputs len: {:?}", inputs.len());
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
    proof: &ProofCompressed,
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
    // Note: this depends on nulifier never matching any of the statetrees.
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
        relay_fee: None,
        root_indices: root_indices.to_vec(),
        utxos,
        proof: Some(proof.clone()),
    };
    InstructionDataTransfer2::serialize(&inputs_struct, &mut inputs).unwrap();
    let instruction_data = crate::instruction::ExecuteCompressedTransaction2 { inputs };
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
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
    let proof = ProofCompressed {
        a: [0u8; 32],
        b: [0u8; 64],
        c: [0u8; 32],
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
            assert_eq!(in_utxos[i], utxo.in_utxo);
        });
    deserialized_instruction_data
        .out_utxos
        .iter()
        .enumerate()
        .for_each(|(i, utxo)| {
            assert_eq!(out_utxos[i], utxo.out_utxo);
        });
    assert_eq!(deserialized_instruction_data.in_utxos.len(), 2);
    assert_eq!(deserialized_instruction_data.out_utxos.len(), 2);
    assert_eq!(deserialized_instruction_data.root_indices, root_indices);
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
    assert_eq!(instruction.accounts[0], AccountMeta::new(payer, true));
    assert_eq!(
        deserialized_instruction_data.in_utxos[0].index_nullifier_array_account,
        1
    );
    assert_eq!(
        deserialized_instruction_data.in_utxos[1].index_nullifier_array_account,
        1
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.in_utxos[0].index_mt_account as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.in_utxos[1].index_mt_account as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.in_utxos[0].index_nullifier_array_account as usize],
        AccountMeta::new(nullifier_array_pubkey, false)
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.in_utxos[1].index_nullifier_array_account as usize],
        AccountMeta::new(nullifier_array_pubkey, false)
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.out_utxos[0].index_mt_account as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
    assert_eq!(
        instruction.accounts
            [6 + deserialized_instruction_data.out_utxos[1].index_mt_account as usize],
        AccountMeta::new(merkle_tree_pubkey, false)
    );
}
