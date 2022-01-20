use crate::tokio::time::timeout;
use light_protocol_core::{
    groth16_verifier::{miller_loop::state::*, parsers::*, prepare_inputs::state::*},
    poseidon_merkle_tree::mt_state::{HashBytes, MerkleTree, MERKLE_TREE_ACC_BYTES},
    process_instruction,
    utils::prepared_verifying_key::*,
};
use solana_program::program_pack::Pack;

use serde_json::{Result, Value};
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};

use solana_program_test::ProgramTestContext;
use std::convert::TryInto;
use std::{fs, time};
mod fe_onchain_test;
mod test_utils;
use crate::test_utils::tests::*;
use ark_ec::ProjectiveCurve;
use ark_groth16::{prepare_inputs, prepare_verifying_key};
//mod tests::fe_onchain_test;
//use crate::fe_onchain_test;
mod mt_onchain_test;
mod pi_onchain_test;
// mod fe_offchain_test;
// use crate::fe_offchain_test::tests::get_public_inputs_from_bytes_254;

pub async fn create_and_start_program_var(
    accounts: &Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "light_protocol_core",
        *program_id,
        processor!(process_instruction),
    );
    println!("accounts {:?}", accounts);
    for (pubkey, size, data) in accounts.iter() {
        println!("accounts {:?}, {:?}, {:?}", pubkey, size, data);

        let mut account = Account::new(10000000000, *size, &program_id);
        match data {
            Some(d) => (account.data = d.clone()),
            None => (),
        }
        program_test.add_account(**pubkey, account);
        println!("added account {:?}", **pubkey);
    }

    let mut program_context = program_test.start_with_context().await;
    let mut transaction = solana_sdk::system_transaction::transfer(
        &program_context.payer,
        &signer_pubkey,
        10000000000000,
        program_context.last_blockhash,
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    let _res_request = program_context
        .banks_client
        .process_transaction(transaction)
        .await;

    program_context
}

pub async fn restart_program(
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
    mut program_context: ProgramTestContext,
) -> ProgramTestContext {
    for (pubkey, _, current_data) in accounts_vector.iter_mut() {
        let account = program_context
            .banks_client
            .get_account(**pubkey)
            .await
            .expect("get_account")
            .unwrap();
        *current_data = Some(account.data.to_vec());
    }
    // accounts_vector[1].2 = Some(storage_account.data.to_vec());
    let program_context_new =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    program_context_new
}

#[test]
fn check_pvk_integrity() {
    let pvk_unprepped = get_vk_from_file().unwrap();
    let pvk = prepare_verifying_key(&pvk_unprepped);
    assert_eq!(get_gamma_abc_g1_0(), pvk.vk.gamma_abc_g1[0]);
    assert_eq!(get_gamma_abc_g1_1(), pvk.vk.gamma_abc_g1[1]);
    assert_eq!(get_gamma_abc_g1_2(), pvk.vk.gamma_abc_g1[2]);
    assert_eq!(get_gamma_abc_g1_3(), pvk.vk.gamma_abc_g1[3]);
    assert_eq!(get_gamma_abc_g1_4(), pvk.vk.gamma_abc_g1[4]);
    assert_eq!(get_gamma_abc_g1_5(), pvk.vk.gamma_abc_g1[5]);
    assert_eq!(get_gamma_abc_g1_6(), pvk.vk.gamma_abc_g1[6]);
    assert_eq!(get_gamma_abc_g1_7(), pvk.vk.gamma_abc_g1[7]);
    assert_eq!(get_gamma_g2_neg_pc_0(), pvk.gamma_g2_neg_pc.ell_coeffs[0]);
    assert_eq!(get_gamma_g2_neg_pc_1(), pvk.gamma_g2_neg_pc.ell_coeffs[1]);
    assert_eq!(get_gamma_g2_neg_pc_2(), pvk.gamma_g2_neg_pc.ell_coeffs[2]);
    assert_eq!(get_gamma_g2_neg_pc_3(), pvk.gamma_g2_neg_pc.ell_coeffs[3]);
    assert_eq!(get_gamma_g2_neg_pc_4(), pvk.gamma_g2_neg_pc.ell_coeffs[4]);
    assert_eq!(get_gamma_g2_neg_pc_5(), pvk.gamma_g2_neg_pc.ell_coeffs[5]);
    assert_eq!(get_gamma_g2_neg_pc_6(), pvk.gamma_g2_neg_pc.ell_coeffs[6]);
    assert_eq!(get_gamma_g2_neg_pc_7(), pvk.gamma_g2_neg_pc.ell_coeffs[7]);
    assert_eq!(get_gamma_g2_neg_pc_8(), pvk.gamma_g2_neg_pc.ell_coeffs[8]);
    assert_eq!(get_gamma_g2_neg_pc_9(), pvk.gamma_g2_neg_pc.ell_coeffs[9]);
    assert_eq!(get_gamma_g2_neg_pc_10(), pvk.gamma_g2_neg_pc.ell_coeffs[10]);
    assert_eq!(get_gamma_g2_neg_pc_11(), pvk.gamma_g2_neg_pc.ell_coeffs[11]);
    assert_eq!(get_gamma_g2_neg_pc_12(), pvk.gamma_g2_neg_pc.ell_coeffs[12]);
    assert_eq!(get_gamma_g2_neg_pc_13(), pvk.gamma_g2_neg_pc.ell_coeffs[13]);
    assert_eq!(get_gamma_g2_neg_pc_14(), pvk.gamma_g2_neg_pc.ell_coeffs[14]);
    assert_eq!(get_gamma_g2_neg_pc_15(), pvk.gamma_g2_neg_pc.ell_coeffs[15]);
    assert_eq!(get_gamma_g2_neg_pc_16(), pvk.gamma_g2_neg_pc.ell_coeffs[16]);
    assert_eq!(get_gamma_g2_neg_pc_17(), pvk.gamma_g2_neg_pc.ell_coeffs[17]);
    assert_eq!(get_gamma_g2_neg_pc_18(), pvk.gamma_g2_neg_pc.ell_coeffs[18]);
    assert_eq!(get_gamma_g2_neg_pc_19(), pvk.gamma_g2_neg_pc.ell_coeffs[19]);
    assert_eq!(get_gamma_g2_neg_pc_20(), pvk.gamma_g2_neg_pc.ell_coeffs[20]);
    assert_eq!(get_gamma_g2_neg_pc_21(), pvk.gamma_g2_neg_pc.ell_coeffs[21]);
    assert_eq!(get_gamma_g2_neg_pc_22(), pvk.gamma_g2_neg_pc.ell_coeffs[22]);
    assert_eq!(get_gamma_g2_neg_pc_23(), pvk.gamma_g2_neg_pc.ell_coeffs[23]);
    assert_eq!(get_gamma_g2_neg_pc_24(), pvk.gamma_g2_neg_pc.ell_coeffs[24]);
    assert_eq!(get_gamma_g2_neg_pc_25(), pvk.gamma_g2_neg_pc.ell_coeffs[25]);
    assert_eq!(get_gamma_g2_neg_pc_26(), pvk.gamma_g2_neg_pc.ell_coeffs[26]);
    assert_eq!(get_gamma_g2_neg_pc_27(), pvk.gamma_g2_neg_pc.ell_coeffs[27]);
    assert_eq!(get_gamma_g2_neg_pc_28(), pvk.gamma_g2_neg_pc.ell_coeffs[28]);
    assert_eq!(get_gamma_g2_neg_pc_29(), pvk.gamma_g2_neg_pc.ell_coeffs[29]);
    assert_eq!(get_gamma_g2_neg_pc_30(), pvk.gamma_g2_neg_pc.ell_coeffs[30]);
    assert_eq!(get_gamma_g2_neg_pc_31(), pvk.gamma_g2_neg_pc.ell_coeffs[31]);
    assert_eq!(get_gamma_g2_neg_pc_32(), pvk.gamma_g2_neg_pc.ell_coeffs[32]);
    assert_eq!(get_gamma_g2_neg_pc_33(), pvk.gamma_g2_neg_pc.ell_coeffs[33]);
    assert_eq!(get_gamma_g2_neg_pc_34(), pvk.gamma_g2_neg_pc.ell_coeffs[34]);
    assert_eq!(get_gamma_g2_neg_pc_35(), pvk.gamma_g2_neg_pc.ell_coeffs[35]);
    assert_eq!(get_gamma_g2_neg_pc_36(), pvk.gamma_g2_neg_pc.ell_coeffs[36]);
    assert_eq!(get_gamma_g2_neg_pc_37(), pvk.gamma_g2_neg_pc.ell_coeffs[37]);
    assert_eq!(get_gamma_g2_neg_pc_38(), pvk.gamma_g2_neg_pc.ell_coeffs[38]);
    assert_eq!(get_gamma_g2_neg_pc_39(), pvk.gamma_g2_neg_pc.ell_coeffs[39]);
    assert_eq!(get_gamma_g2_neg_pc_40(), pvk.gamma_g2_neg_pc.ell_coeffs[40]);
    assert_eq!(get_gamma_g2_neg_pc_41(), pvk.gamma_g2_neg_pc.ell_coeffs[41]);
    assert_eq!(get_gamma_g2_neg_pc_42(), pvk.gamma_g2_neg_pc.ell_coeffs[42]);
    assert_eq!(get_gamma_g2_neg_pc_43(), pvk.gamma_g2_neg_pc.ell_coeffs[43]);
    assert_eq!(get_gamma_g2_neg_pc_44(), pvk.gamma_g2_neg_pc.ell_coeffs[44]);
    assert_eq!(get_gamma_g2_neg_pc_45(), pvk.gamma_g2_neg_pc.ell_coeffs[45]);
    assert_eq!(get_gamma_g2_neg_pc_46(), pvk.gamma_g2_neg_pc.ell_coeffs[46]);
    assert_eq!(get_gamma_g2_neg_pc_47(), pvk.gamma_g2_neg_pc.ell_coeffs[47]);
    assert_eq!(get_gamma_g2_neg_pc_48(), pvk.gamma_g2_neg_pc.ell_coeffs[48]);
    assert_eq!(get_gamma_g2_neg_pc_49(), pvk.gamma_g2_neg_pc.ell_coeffs[49]);
    assert_eq!(get_gamma_g2_neg_pc_50(), pvk.gamma_g2_neg_pc.ell_coeffs[50]);
    assert_eq!(get_gamma_g2_neg_pc_51(), pvk.gamma_g2_neg_pc.ell_coeffs[51]);
    assert_eq!(get_gamma_g2_neg_pc_52(), pvk.gamma_g2_neg_pc.ell_coeffs[52]);
    assert_eq!(get_gamma_g2_neg_pc_53(), pvk.gamma_g2_neg_pc.ell_coeffs[53]);
    assert_eq!(get_gamma_g2_neg_pc_54(), pvk.gamma_g2_neg_pc.ell_coeffs[54]);
    assert_eq!(get_gamma_g2_neg_pc_55(), pvk.gamma_g2_neg_pc.ell_coeffs[55]);
    assert_eq!(get_gamma_g2_neg_pc_56(), pvk.gamma_g2_neg_pc.ell_coeffs[56]);
    assert_eq!(get_gamma_g2_neg_pc_57(), pvk.gamma_g2_neg_pc.ell_coeffs[57]);
    assert_eq!(get_gamma_g2_neg_pc_58(), pvk.gamma_g2_neg_pc.ell_coeffs[58]);
    assert_eq!(get_gamma_g2_neg_pc_59(), pvk.gamma_g2_neg_pc.ell_coeffs[59]);
    assert_eq!(get_gamma_g2_neg_pc_60(), pvk.gamma_g2_neg_pc.ell_coeffs[60]);
    assert_eq!(get_gamma_g2_neg_pc_61(), pvk.gamma_g2_neg_pc.ell_coeffs[61]);
    assert_eq!(get_gamma_g2_neg_pc_62(), pvk.gamma_g2_neg_pc.ell_coeffs[62]);
    assert_eq!(get_gamma_g2_neg_pc_63(), pvk.gamma_g2_neg_pc.ell_coeffs[63]);
    assert_eq!(get_gamma_g2_neg_pc_64(), pvk.gamma_g2_neg_pc.ell_coeffs[64]);
    assert_eq!(get_gamma_g2_neg_pc_65(), pvk.gamma_g2_neg_pc.ell_coeffs[65]);
    assert_eq!(get_gamma_g2_neg_pc_66(), pvk.gamma_g2_neg_pc.ell_coeffs[66]);
    assert_eq!(get_gamma_g2_neg_pc_67(), pvk.gamma_g2_neg_pc.ell_coeffs[67]);
    assert_eq!(get_gamma_g2_neg_pc_68(), pvk.gamma_g2_neg_pc.ell_coeffs[68]);
    assert_eq!(get_gamma_g2_neg_pc_69(), pvk.gamma_g2_neg_pc.ell_coeffs[69]);
    assert_eq!(get_gamma_g2_neg_pc_70(), pvk.gamma_g2_neg_pc.ell_coeffs[70]);
    assert_eq!(get_gamma_g2_neg_pc_71(), pvk.gamma_g2_neg_pc.ell_coeffs[71]);
    assert_eq!(get_gamma_g2_neg_pc_72(), pvk.gamma_g2_neg_pc.ell_coeffs[72]);
    assert_eq!(get_gamma_g2_neg_pc_73(), pvk.gamma_g2_neg_pc.ell_coeffs[73]);
    assert_eq!(get_gamma_g2_neg_pc_74(), pvk.gamma_g2_neg_pc.ell_coeffs[74]);
    assert_eq!(get_gamma_g2_neg_pc_75(), pvk.gamma_g2_neg_pc.ell_coeffs[75]);
    assert_eq!(get_gamma_g2_neg_pc_76(), pvk.gamma_g2_neg_pc.ell_coeffs[76]);
    assert_eq!(get_gamma_g2_neg_pc_77(), pvk.gamma_g2_neg_pc.ell_coeffs[77]);
    assert_eq!(get_gamma_g2_neg_pc_78(), pvk.gamma_g2_neg_pc.ell_coeffs[78]);
    assert_eq!(get_gamma_g2_neg_pc_79(), pvk.gamma_g2_neg_pc.ell_coeffs[79]);
    assert_eq!(get_gamma_g2_neg_pc_80(), pvk.gamma_g2_neg_pc.ell_coeffs[80]);
    assert_eq!(get_gamma_g2_neg_pc_81(), pvk.gamma_g2_neg_pc.ell_coeffs[81]);
    assert_eq!(get_gamma_g2_neg_pc_82(), pvk.gamma_g2_neg_pc.ell_coeffs[82]);
    assert_eq!(get_gamma_g2_neg_pc_83(), pvk.gamma_g2_neg_pc.ell_coeffs[83]);
    assert_eq!(get_gamma_g2_neg_pc_84(), pvk.gamma_g2_neg_pc.ell_coeffs[84]);
    assert_eq!(get_gamma_g2_neg_pc_85(), pvk.gamma_g2_neg_pc.ell_coeffs[85]);
    assert_eq!(get_gamma_g2_neg_pc_86(), pvk.gamma_g2_neg_pc.ell_coeffs[86]);
    assert_eq!(get_gamma_g2_neg_pc_87(), pvk.gamma_g2_neg_pc.ell_coeffs[87]);
    assert_eq!(get_gamma_g2_neg_pc_88(), pvk.gamma_g2_neg_pc.ell_coeffs[88]);
    assert_eq!(get_gamma_g2_neg_pc_89(), pvk.gamma_g2_neg_pc.ell_coeffs[89]);
    assert_eq!(get_gamma_g2_neg_pc_90(), pvk.gamma_g2_neg_pc.ell_coeffs[90]);
    assert_eq!(get_delta_g2_neg_pc_0(), pvk.delta_g2_neg_pc.ell_coeffs[0]);
    assert_eq!(get_delta_g2_neg_pc_1(), pvk.delta_g2_neg_pc.ell_coeffs[1]);
    assert_eq!(get_delta_g2_neg_pc_2(), pvk.delta_g2_neg_pc.ell_coeffs[2]);
    assert_eq!(get_delta_g2_neg_pc_3(), pvk.delta_g2_neg_pc.ell_coeffs[3]);
    assert_eq!(get_delta_g2_neg_pc_4(), pvk.delta_g2_neg_pc.ell_coeffs[4]);
    assert_eq!(get_delta_g2_neg_pc_5(), pvk.delta_g2_neg_pc.ell_coeffs[5]);
    assert_eq!(get_delta_g2_neg_pc_6(), pvk.delta_g2_neg_pc.ell_coeffs[6]);
    assert_eq!(get_delta_g2_neg_pc_7(), pvk.delta_g2_neg_pc.ell_coeffs[7]);
    assert_eq!(get_delta_g2_neg_pc_8(), pvk.delta_g2_neg_pc.ell_coeffs[8]);
    assert_eq!(get_delta_g2_neg_pc_9(), pvk.delta_g2_neg_pc.ell_coeffs[9]);
    assert_eq!(get_delta_g2_neg_pc_10(), pvk.delta_g2_neg_pc.ell_coeffs[10]);
    assert_eq!(get_delta_g2_neg_pc_11(), pvk.delta_g2_neg_pc.ell_coeffs[11]);
    assert_eq!(get_delta_g2_neg_pc_12(), pvk.delta_g2_neg_pc.ell_coeffs[12]);
    assert_eq!(get_delta_g2_neg_pc_13(), pvk.delta_g2_neg_pc.ell_coeffs[13]);
    assert_eq!(get_delta_g2_neg_pc_14(), pvk.delta_g2_neg_pc.ell_coeffs[14]);
    assert_eq!(get_delta_g2_neg_pc_15(), pvk.delta_g2_neg_pc.ell_coeffs[15]);
    assert_eq!(get_delta_g2_neg_pc_16(), pvk.delta_g2_neg_pc.ell_coeffs[16]);
    assert_eq!(get_delta_g2_neg_pc_17(), pvk.delta_g2_neg_pc.ell_coeffs[17]);
    assert_eq!(get_delta_g2_neg_pc_18(), pvk.delta_g2_neg_pc.ell_coeffs[18]);
    assert_eq!(get_delta_g2_neg_pc_19(), pvk.delta_g2_neg_pc.ell_coeffs[19]);
    assert_eq!(get_delta_g2_neg_pc_20(), pvk.delta_g2_neg_pc.ell_coeffs[20]);
    assert_eq!(get_delta_g2_neg_pc_21(), pvk.delta_g2_neg_pc.ell_coeffs[21]);
    assert_eq!(get_delta_g2_neg_pc_22(), pvk.delta_g2_neg_pc.ell_coeffs[22]);
    assert_eq!(get_delta_g2_neg_pc_23(), pvk.delta_g2_neg_pc.ell_coeffs[23]);
    assert_eq!(get_delta_g2_neg_pc_24(), pvk.delta_g2_neg_pc.ell_coeffs[24]);
    assert_eq!(get_delta_g2_neg_pc_25(), pvk.delta_g2_neg_pc.ell_coeffs[25]);
    assert_eq!(get_delta_g2_neg_pc_26(), pvk.delta_g2_neg_pc.ell_coeffs[26]);
    assert_eq!(get_delta_g2_neg_pc_27(), pvk.delta_g2_neg_pc.ell_coeffs[27]);
    assert_eq!(get_delta_g2_neg_pc_28(), pvk.delta_g2_neg_pc.ell_coeffs[28]);
    assert_eq!(get_delta_g2_neg_pc_29(), pvk.delta_g2_neg_pc.ell_coeffs[29]);
    assert_eq!(get_delta_g2_neg_pc_30(), pvk.delta_g2_neg_pc.ell_coeffs[30]);
    assert_eq!(get_delta_g2_neg_pc_31(), pvk.delta_g2_neg_pc.ell_coeffs[31]);
    assert_eq!(get_delta_g2_neg_pc_32(), pvk.delta_g2_neg_pc.ell_coeffs[32]);
    assert_eq!(get_delta_g2_neg_pc_33(), pvk.delta_g2_neg_pc.ell_coeffs[33]);
    assert_eq!(get_delta_g2_neg_pc_34(), pvk.delta_g2_neg_pc.ell_coeffs[34]);
    assert_eq!(get_delta_g2_neg_pc_35(), pvk.delta_g2_neg_pc.ell_coeffs[35]);
    assert_eq!(get_delta_g2_neg_pc_36(), pvk.delta_g2_neg_pc.ell_coeffs[36]);
    assert_eq!(get_delta_g2_neg_pc_37(), pvk.delta_g2_neg_pc.ell_coeffs[37]);
    assert_eq!(get_delta_g2_neg_pc_38(), pvk.delta_g2_neg_pc.ell_coeffs[38]);
    assert_eq!(get_delta_g2_neg_pc_39(), pvk.delta_g2_neg_pc.ell_coeffs[39]);
    assert_eq!(get_delta_g2_neg_pc_40(), pvk.delta_g2_neg_pc.ell_coeffs[40]);
    assert_eq!(get_delta_g2_neg_pc_41(), pvk.delta_g2_neg_pc.ell_coeffs[41]);
    assert_eq!(get_delta_g2_neg_pc_42(), pvk.delta_g2_neg_pc.ell_coeffs[42]);
    assert_eq!(get_delta_g2_neg_pc_43(), pvk.delta_g2_neg_pc.ell_coeffs[43]);
    assert_eq!(get_delta_g2_neg_pc_44(), pvk.delta_g2_neg_pc.ell_coeffs[44]);
    assert_eq!(get_delta_g2_neg_pc_45(), pvk.delta_g2_neg_pc.ell_coeffs[45]);
    assert_eq!(get_delta_g2_neg_pc_46(), pvk.delta_g2_neg_pc.ell_coeffs[46]);
    assert_eq!(get_delta_g2_neg_pc_47(), pvk.delta_g2_neg_pc.ell_coeffs[47]);
    assert_eq!(get_delta_g2_neg_pc_48(), pvk.delta_g2_neg_pc.ell_coeffs[48]);
    assert_eq!(get_delta_g2_neg_pc_49(), pvk.delta_g2_neg_pc.ell_coeffs[49]);
    assert_eq!(get_delta_g2_neg_pc_50(), pvk.delta_g2_neg_pc.ell_coeffs[50]);
    assert_eq!(get_delta_g2_neg_pc_51(), pvk.delta_g2_neg_pc.ell_coeffs[51]);
    assert_eq!(get_delta_g2_neg_pc_52(), pvk.delta_g2_neg_pc.ell_coeffs[52]);
    assert_eq!(get_delta_g2_neg_pc_53(), pvk.delta_g2_neg_pc.ell_coeffs[53]);
    assert_eq!(get_delta_g2_neg_pc_54(), pvk.delta_g2_neg_pc.ell_coeffs[54]);
    assert_eq!(get_delta_g2_neg_pc_55(), pvk.delta_g2_neg_pc.ell_coeffs[55]);
    assert_eq!(get_delta_g2_neg_pc_56(), pvk.delta_g2_neg_pc.ell_coeffs[56]);
    assert_eq!(get_delta_g2_neg_pc_57(), pvk.delta_g2_neg_pc.ell_coeffs[57]);
    assert_eq!(get_delta_g2_neg_pc_58(), pvk.delta_g2_neg_pc.ell_coeffs[58]);
    assert_eq!(get_delta_g2_neg_pc_59(), pvk.delta_g2_neg_pc.ell_coeffs[59]);
    assert_eq!(get_delta_g2_neg_pc_60(), pvk.delta_g2_neg_pc.ell_coeffs[60]);
    assert_eq!(get_delta_g2_neg_pc_61(), pvk.delta_g2_neg_pc.ell_coeffs[61]);
    assert_eq!(get_delta_g2_neg_pc_62(), pvk.delta_g2_neg_pc.ell_coeffs[62]);
    assert_eq!(get_delta_g2_neg_pc_63(), pvk.delta_g2_neg_pc.ell_coeffs[63]);
    assert_eq!(get_delta_g2_neg_pc_64(), pvk.delta_g2_neg_pc.ell_coeffs[64]);
    assert_eq!(get_delta_g2_neg_pc_65(), pvk.delta_g2_neg_pc.ell_coeffs[65]);
    assert_eq!(get_delta_g2_neg_pc_66(), pvk.delta_g2_neg_pc.ell_coeffs[66]);
    assert_eq!(get_delta_g2_neg_pc_67(), pvk.delta_g2_neg_pc.ell_coeffs[67]);
    assert_eq!(get_delta_g2_neg_pc_68(), pvk.delta_g2_neg_pc.ell_coeffs[68]);
    assert_eq!(get_delta_g2_neg_pc_69(), pvk.delta_g2_neg_pc.ell_coeffs[69]);
    assert_eq!(get_delta_g2_neg_pc_70(), pvk.delta_g2_neg_pc.ell_coeffs[70]);
    assert_eq!(get_delta_g2_neg_pc_71(), pvk.delta_g2_neg_pc.ell_coeffs[71]);
    assert_eq!(get_delta_g2_neg_pc_72(), pvk.delta_g2_neg_pc.ell_coeffs[72]);
    assert_eq!(get_delta_g2_neg_pc_73(), pvk.delta_g2_neg_pc.ell_coeffs[73]);
    assert_eq!(get_delta_g2_neg_pc_74(), pvk.delta_g2_neg_pc.ell_coeffs[74]);
    assert_eq!(get_delta_g2_neg_pc_75(), pvk.delta_g2_neg_pc.ell_coeffs[75]);
    assert_eq!(get_delta_g2_neg_pc_76(), pvk.delta_g2_neg_pc.ell_coeffs[76]);
    assert_eq!(get_delta_g2_neg_pc_77(), pvk.delta_g2_neg_pc.ell_coeffs[77]);
    assert_eq!(get_delta_g2_neg_pc_78(), pvk.delta_g2_neg_pc.ell_coeffs[78]);
    assert_eq!(get_delta_g2_neg_pc_79(), pvk.delta_g2_neg_pc.ell_coeffs[79]);
    assert_eq!(get_delta_g2_neg_pc_80(), pvk.delta_g2_neg_pc.ell_coeffs[80]);
    assert_eq!(get_delta_g2_neg_pc_81(), pvk.delta_g2_neg_pc.ell_coeffs[81]);
    assert_eq!(get_delta_g2_neg_pc_82(), pvk.delta_g2_neg_pc.ell_coeffs[82]);
    assert_eq!(get_delta_g2_neg_pc_83(), pvk.delta_g2_neg_pc.ell_coeffs[83]);
    assert_eq!(get_delta_g2_neg_pc_84(), pvk.delta_g2_neg_pc.ell_coeffs[84]);
    assert_eq!(get_delta_g2_neg_pc_85(), pvk.delta_g2_neg_pc.ell_coeffs[85]);
    assert_eq!(get_delta_g2_neg_pc_86(), pvk.delta_g2_neg_pc.ell_coeffs[86]);
    assert_eq!(get_delta_g2_neg_pc_87(), pvk.delta_g2_neg_pc.ell_coeffs[87]);
    assert_eq!(get_delta_g2_neg_pc_88(), pvk.delta_g2_neg_pc.ell_coeffs[88]);
    assert_eq!(get_delta_g2_neg_pc_89(), pvk.delta_g2_neg_pc.ell_coeffs[89]);
    assert_eq!(get_delta_g2_neg_pc_90(), pvk.delta_g2_neg_pc.ell_coeffs[90]);
}

pub fn read_test_data() -> Vec<u8> {
    let ix_data_file = fs::read_to_string("./tests/test_data/deposit_0_1_sol.txt")
        .expect("Something went wrong reading the file");
    let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
    let mut ix_data = Vec::new();
    for i in ix_data_json["bytes"][0].as_str().unwrap().split(',') {
        let j = (*i).parse::<u8>();
        match j {
            Ok(x) => (ix_data.push(x)),
            Err(_e) => (),
        }
    }
    println!("{:?}", ix_data);
    ix_data
}

pub fn get_proof_from_bytes(
    proof_bytes: &Vec<u8>,
) -> ark_groth16::data_structures::Proof<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
    let proof_a = parse_x_group_affine_from_bytes(&proof_bytes[0..64].to_vec());
    let proof_b = parse_proof_b_from_bytes(&proof_bytes[64..192].to_vec());
    let proof_c = parse_x_group_affine_from_bytes(&proof_bytes[192..256].to_vec());
    let proof =
        ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
    proof
}

#[tokio::test]
#[ignore]
async fn ml_test_onchain_new() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    // let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data();
    //create pubkey for temporary storage account
    let storage_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    println!(
        "storage_pubkey add seed: {:?}",
        [&ix_data[105..137], &b"storage"[..]]
    );

    // Pick the data we need from the test file. 9.. bc of input structure
    let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
    let proof_bytes = ix_data[233..489].to_vec(); // 256 length
    let pvk_unprepped = get_vk_from_file().unwrap(); //?// TODO: check if same vk
    let pvk = prepare_verifying_key(&pvk_unprepped);
    let proof = get_proof_from_bytes(&proof_bytes);
    let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap(); // TODO: debug
    let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
    // While the native library implementation does the conversion only when the millerloop is called.
    // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
    let as_affine = (prepared_inputs).into_affine();
    let mut affine_bytes = vec![0; 64];
    parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
    // mock account state after prepare_inputs (instruction index = 466)
    let mut account_state = vec![0; 3900];
    // set is_initialized:true
    account_state[0] = 1;
    // for x_1_range alas prepared_inputs.into_affine()
    for (index, i) in affine_bytes.iter().enumerate() {
        account_state[index + 252] = *i;
    }
    // for proof a,b,c
    for (index, i) in proof_bytes.iter().enumerate() {
        account_state[index + 3516] = *i;
    }
    // set current index
    let current_index = 466 as usize;
    for (index, i) in current_index.to_le_bytes().iter().enumerate() {
        account_state[index + 212] = *i;
    }
    // We need to set the signer since otherwise the signer check fails on-chain
    let signer_pubkey_bytes = signer_keypair.to_bytes();
    for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
        account_state[index + 4] = *i;
    }
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&storage_pubkey, 3900, Some(account_state)));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    // Calculate miller_ouput with the ark library. Will be used to compare the
    // on-chain output with.
    let miller_output =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
            [
                (proof.a.into(), proof.b.into()),
                (
                    (prepared_inputs).into_affine().into(),
                    pvk.gamma_g2_neg_pc.clone(),
                ),
                (proof.c.into(), pvk.delta_g2_neg_pc.clone()),
            ]
            .iter(),
        );

    /*
     *
     *
     *Miller loop
     *
     *
     */
    // Just making sure we're not having the exact same ix_data/ix in the same block.
    // Since the runtime dedupes any exactly equivalent ix within the same block.
    // We need program restart logic here since we're firing 300+ ix and
    // the program_context seems to melt down every couple of hundred ix.
    // It basically just picks up the account state where it left off and restarts the client
    let mut i = 0usize;
    for _id in 0..430usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(signer_pubkey, true),
                        AccountMeta::new(storage_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);
            let res_request = timeout(
                time::Duration::from_millis(500),
                program_context
                    .banks_client
                    .process_transaction(transaction),
            )
            .await;
            match res_request {
                Ok(_) => success = true,
                Err(_e) => {
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;
                    program_context = restart_program(
                        &mut accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
        }
        i += 1;
    }

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
    // TODO:compare with miller_output
    let mut ref_f = vec![0; 384];
    parse_f_to_bytes(miller_output, &mut ref_f);
    assert_eq!(
        account_data.f_range, ref_f,
        "onchain f result != reference f (hardcoded from lib call)"
    );
    println!("onchain test success");
}

#[tokio::test]
async fn pi_test_onchain_new() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    //create pubkey for temporary storage account
    // let storage_pubkey = Pubkey::new_unique();

    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data();
    let storage_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    println!(
        "storage_pubkey add seed: {:?}",
        [&ix_data[105..137], &b"storage"[..]]
    );
    println!("storage_pubkey: {:?}", storage_pubkey);
    // Pick the data we need from the test file. 9.. bc of input structure
    let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
    let pvk_unprepped = get_vk_from_file().unwrap();
    let pvk = prepare_verifying_key(&pvk_unprepped);

    let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap();
    let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
    // While the native library implementation does the conversion only when the millerloop is called.
    // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
    let as_affine = (prepared_inputs).into_affine();
    let mut affine_bytes = vec![0; 64];
    parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);

    let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pubkey, 16657, None));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    // Initialize MerkleTree account
    // TODO: this needed?
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);

    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let merkle_tree_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    /*
     *
     *
     * Send data to chain
     *
     *
     */
    //first instruction + prepare inputs id + 7 public inputs in bytes = 226 bytes

    //sends bytes
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..].to_vec(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
                AccountMeta::new(
                    Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                    false,
                ),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    accounts_vector.push((&storage_pubkey, 3900, None));

    /*
     *
     *
     * check merkle root
     *
     *
     */

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..20].to_vec(), //random
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    /*
     *
     *
     *Prepare inputs
     *
     *
     */
    // Just making sure we're not having the exact same ix_data/ix in the same block.
    // Since the runtime dedupes any exactly equivalent ix within the same block.
    // We need program restart logic here since we're firing 300+ ix and
    // the program_context seems to melt down every couple of hundred ix.
    // It basically just picks up the account state where it left off and restarts the client
    let mut i = 0usize;
    for _id in 0..464usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(signer_pubkey, true),
                        // AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(storage_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);
            let res_request = timeout(
                time::Duration::from_millis(500),
                program_context
                    .banks_client
                    .process_transaction(transaction),
            )
            .await;
            match res_request {
                Ok(_) => success = true,
                Err(_e) => {
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;
                    program_context = restart_program(
                        &mut accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
        }
        i += 1;
    }

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let account_data = PiBytes::unpack(&storage_account.data.clone()).unwrap();
    assert_eq!(
        account_data.x_1_range, affine_bytes,
        "onchain pi result != reference pi.into:affine() (hardcoded from lib call)"
    );
    println!("onchain test success");
}
