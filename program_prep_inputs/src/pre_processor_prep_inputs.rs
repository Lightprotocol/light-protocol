use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
};
use crate::state_prep_inputs::{PrepareInputsBytes, PoseidonHashBytesPrepInputs};
use crate::processor_prepare_inputs::*;
use ark_ec;
use ark_ff;
use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ff::biginteger::{BigInteger256,BigInteger384};
use ark_ff::{Fp256, Fp384};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use crate::ranges_prepare_inputs::*;
use crate::parsers_prepare_inputs::*;
use crate::state_merkle_tree_roots;
use crate::poseidon_processor;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};


pub fn _pre_process_instruction_prep_inputs(_instruction_data: &[u8], accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let  signing_account = next_account_info(account)?;
    // the remaining storage accounts are being pulled inside each loop
    let complete_instruction_order_fill_p: Vec<u8> = vec![
    40,
    0, 1, 2, 3 ,4 ,5 ,6 ,7 ,8 ,9 ,10 ,11 ,12 ,13 ,14 ,15 ,16 ,17 ,18 ,30 ,31 ,32 ,33 ,19 ,20 ,21, 24, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23,
    41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 46, 41, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44,  44, 46, 41,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45,  45, 46, 47, 48];
    //msg!("len complete_instruction_order_fill_p {}",complete_instruction_order_fill_p.len() );
    let account_prepare_inputs = next_account_info(account)?;

    let mut account_data = PrepareInputsBytes::unpack(&account_prepare_inputs.data.borrow())?;
    assert!(account_data.current_instruction_index < 1086, "Preparing inputs finished");
    // prepare inputs and store p2 in acc
    //assert_eq!(complete_instruction_order_fill_p[account_data.current_instruction_index], _instruction_data[0], "wrong index");
    msg!("Executing instruction: {}", complete_instruction_order_fill_p[account_data.current_instruction_index]);
    //msg!("Should execute instruction: {}", _instruction_data[0]);
    if _instruction_data[1] == 3 {

        if complete_instruction_order_fill_p[account_data.current_instruction_index] == 47 { // into affine
            let g_ic : ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters> = parse_group_projective_from_bytes(&account_data.g_ic_x_range, &account_data.g_ic_y_range, &account_data.g_ic_z_range); // 15k

            // CUSTOM ALT
            let zinv = ark_ff::Field::inverse(&g_ic.z).unwrap();

            let g_ic_with_zinv :ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters>  = ark_ec::short_weierstrass_jacobian::GroupProjective::new(
                g_ic.x,
                g_ic.y,
                zinv
            );
            parse_group_projective_to_bytes(g_ic_with_zinv,&mut account_data.g_ic_x_range, &mut account_data.g_ic_y_range, &mut account_data.g_ic_z_range);
            account_data.changed_variables[G_IC_Z_RANGE_INDEX] = true;

        } else if complete_instruction_order_fill_p[account_data.current_instruction_index] == 48 { // rest
            let g_ic : ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters> = parse_group_projective_from_bytes(&account_data.g_ic_x_range, &account_data.g_ic_y_range, &account_data.g_ic_z_range); // 15k
            let zinv_squared = ark_ff::Field::square(&g_ic.z); // zinv.square();


            let x = g_ic.x * &zinv_squared;

            let y = g_ic.y * &(zinv_squared * &g_ic.z); // * zinv
            sol_log_compute_units();
            let mut g_ic_affine :ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters> = ark_ec::short_weierstrass_jacobian::GroupAffine::new(x, y, false);
            parse_x_group_affine_to_bytes(g_ic_affine, &mut account_data.x_1_range); // overwrite x1range w: 5066
            account_data.changed_variables[X_1_RANGE_INDEX] = true;

        }
        else if complete_instruction_order_fill_p[account_data.current_instruction_index] < 40 {
            let mut account_data = PoseidonHashBytesPrepInputs::unpack(&account_prepare_inputs.data.borrow())?;

            poseidon_processor::processor(complete_instruction_order_fill_p[account_data.current_instruction_index], &mut account_data);
            /*
            //absorbing
            if complete_instruction_order_fill_p[account_data.current_instruction_index] == 1 || complete_instruction_order_fill_p[account_data.current_instruction_index] == 2 {
                sol_log_compute_units();
                msg!("here1");
                sol_log_compute_units();
                msg!("here2");
            } else {
                sol_log_compute_units();
                msg!("here1");
                poseidon_processor::processor(complete_instruction_order_fill_p[account_data.current_instruction_index], &mut account_data);
                sol_log_compute_units();
                msg!("here2");
            }*/
            //account_data.current_instruction_index += 1;
            PoseidonHashBytesPrepInputs::pack_into_slice(&account_data, &mut account_prepare_inputs.data.borrow_mut());
            //assert_eq!( signing_account.key.to_bytes().to_vec(), account_data.signing_address);
            //return Ok(());
        }
        else {

            let mut current_index = 99;
            if complete_instruction_order_fill_p[account_data.current_instruction_index] > 41 && complete_instruction_order_fill_p[account_data.current_instruction_index] < 46 {
                current_index = _instruction_data[2];
            }

            let mut public_inputs : Vec<ark_ff::Fp256<ark_bls12_381::FrParameters>> = vec![];

            //init instruction && check for root hash existence
            if complete_instruction_order_fill_p[account_data.current_instruction_index] == 40 {
                let input = array_ref![_instruction_data,0, 210];
                let (
                    unused_prior,
                    input_1,
                    input_2,
                    input_3,
                    input_4,

                    amount_bytes,
                    relayer_refund_bytes,
                    to_address_bytes,
                    signing_address_bytes,
                ) = array_refs!(input, 2, 32, 32, 32, 32, 8, 8, 32, 32);

                //commit hash to be replaced by data_hash
                let input1 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&_instruction_data[2..34]).unwrap();
                //nullifier hash
                let input2 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&_instruction_data[34..66]).unwrap();
                account_data.nullifier_hash = _instruction_data[34..66].to_vec();
                //msg!("nullifier_hash: {:?}", account_data.nullifier_hash);
                //root hash
                let input3 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&_instruction_data[66..98]).unwrap();
                account_data.root_hash = input_3.to_vec();
                //msg!("root: {:?}", account_data.root_hash);
                //tx integrity hash
                let input4 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&_instruction_data[98..130]).unwrap();

                account_data.tx_integrity_hash = input_4.to_vec();

                account_data.amount             = amount_bytes.to_vec();
                msg!("amount: {:?}",account_data.amount   );

                account_data.relayer_refund     = relayer_refund_bytes.to_vec();
                msg!("relayer_refund: {:?}",account_data.relayer_refund   );

                account_data.to_address         = to_address_bytes.to_vec();
                msg!("to_address: {:?}",account_data.to_address   );
                assert_eq!(*signing_account.key, solana_program::pubkey::Pubkey::new(signing_address_bytes));

                account_data.signing_address    =  signing_account.key.to_bytes().to_vec();
                msg!("signing_address: {:?}",account_data.signing_address   );

                let account_merkle_tree = next_account_info(account)?;
                //local host let merkletree_acc_bytes: [u8;32] = [251, 30, 194, 174, 168, 85, 13, 188, 134, 0, 17, 157, 187, 32, 113, 104, 134, 138, 82, 128, 95, 206, 76, 34, 177, 163, 246, 27, 109, 207, 2, 85];
                //let merkletree_acc_bytes: [u8;32] =[14,   6,  73, 209, 163, 244, 108,  152, 171, 216,  16, 214, 160, 160,  167, 228, 175, 183, 171, 175, 131,  235, 227, 100, 101, 217, 250,  96,  173,  34,  59,  62];
                msg!("Merkle_tree_bytes: {:?}", account_merkle_tree.key.to_bytes());
                assert_eq!(*account_merkle_tree.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree_roots::MERKLE_TREE_ACC_BYTES[..]));
                state_merkle_tree_roots::check_root_hash_exists(account_merkle_tree, _instruction_data[66..98].to_vec(), &mut account_data.found_root);
                assert_eq!(account_data.found_root, 1u8);
                //for reference
                // let mut sponge_tx_integrity = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_sponge_params());
                //
                // for byte in [ amount, relayer_refund].concat().iter() {
                //     sponge_tx_integrity.absorb(byte);
                // }
                // for byte in [ to_address, relayer_address].concat().iter() {
                //     sponge_tx_integrity.absorb(byte);
                // }
                //
                // let mut tx_integrity_hash = sponge_tx_integrity.squeeze_field_elements::<ark_ed_on_bls12_381::Fq>(1)[0].clone();

                msg!("account_data.found_root {}", account_data.found_root);

                public_inputs = vec![input1,input2,input3,input4];

                //save public inputs as constant<w>

                for i in 0..11 {
                    account_data.changed_constants[i] = true;
                }
                // fixed inputs
                //assert_eq!(&_instruction_data[2..130], &[151, 85, 62, 182, 26, 238, 149, 115, 117, 89, 25, 56, 176, 33, 124, 54, 229, 133, 85, 3, 220, 179, 228, 88, 14, 137, 72, 68, 230, 230, 25, 74, 132, 1, 14, 72, 111, 54, 123, 94, 251, 147, 244, 75, 86, 228, 18, 126, 214, 240, 54, 15, 174, 215, 153, 99, 84, 160, 10, 189, 134, 166, 186, 7, 186, 11, 250, 107, 131, 86, 119, 78, 239, 31, 50, 120, 132, 189, 175, 67, 30, 6, 80, 159, 190, 145, 23, 2, 253, 30, 141, 111, 155, 114, 43, 46, 135, 53, 48, 239, 128, 88, 250, 198, 168, 133, 132, 213, 193, 140, 155, 186, 110, 136, 116, 194, 162, 215, 89, 167, 96, 40, 16, 127, 67, 203, 177, 47]);
            }

            msg!("instruction procssing...current index: {:?}, inputs: {:?}",current_index, public_inputs);
            _process_instruction_prepare_inputs( complete_instruction_order_fill_p[account_data.current_instruction_index], &mut account_data, &public_inputs, current_index.into());

        }
        //msg!("changed_variables: {:?}", account_data.changed_variables);
        //msg!("changed_constants: {:?}", account_data.changed_constants);
        //assert_eq!( signing_account.key.to_bytes().to_vec(), account_data.signing_address);
        assert_eq!(* signing_account.key, solana_program::pubkey::Pubkey::new(&account_data.signing_address));
        assert_eq!(1u8, account_data.found_root);

        account_data.current_instruction_index += 1;
        msg!("current_instruction_index: {}", account_data.current_instruction_index);
        PrepareInputsBytes::pack_into_slice(&account_data, &mut account_prepare_inputs.data.borrow_mut());

    }
    Ok(())
}
