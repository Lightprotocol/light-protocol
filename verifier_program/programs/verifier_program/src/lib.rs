pub mod groth16_verifier;
pub mod utils;
use anchor_lang::solana_program::system_program;

// use crate::groth16_verifier::prepare_inputs::state::{
//     CreateVerifierState
// };
use groth16_verifier::prepare_inputs::*;
use groth16_verifier::miller_loop::*;
use crate::groth16_verifier::final_exponentiation_process_instruction;
use crate::groth16_verifier::FinalExponentiationState;

use ark_ec::bn::g2::G2HomProjective;
use crate::groth16_verifier::parse_r_to_bytes;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use ark_ff::Fp2;
use ark_std::One;
use crate::groth16_verifier::FinalExponentiationComputeState;
use crate::groth16_verifier::parsers::*;

use anchor_lang::prelude::*;
use solana_program::log::sol_log_compute_units;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
     use super::*;



     pub fn create_tmp_account(ctx: Context<CreateInputsState>,
            proof:        [u8;256],

            root_hash:          [u8;32],
            amount:             [u8;32],
            tx_integrity_hash:  [u8;32],
            nullifier0:         [u8;32],
            nullifier1:         [u8;32],
            leaf_right:         [u8;32],
            leaf_left:          [u8;32],
            // relayer_fee:        bytes.slice(264,272),
            // ext_sol_amount:     bytes.slice(272,304),
            // verifier_index:     bytes.slice(304,312),
            // merkle_tree_index:  bytes.slice(312,320),
            recipient:          [u8;32],
            ext_amount:         [u8;8],
            relayer:            [u8;32],
            fee:                [u8;8],
            merkle_tree_pda_pubkey:[u8;32],
            _encrypted_utxos:    [u8;256],//,bytes.slice(593,593+222),
            // prepared_inputs: [u8;64],
            merkle_tree_index:  [u8;1],
     ) -> Result<()> {


         let tmp_account= &mut ctx.accounts.verifier_state.load_init()?;
         tmp_account.signing_address = ctx.accounts.signing_address.key();
         // let x: u8 = tmp_account;
         tmp_account.root_hash = root_hash.clone();
         msg!("root_hash {:?}", tmp_account.root_hash);
         assert_eq!(tmp_account.root_hash, root_hash);
         tmp_account.amount = amount.clone();

         tmp_account.merkle_tree_tmp_account = Pubkey::new(&merkle_tree_pda_pubkey).clone();
         tmp_account.merkle_tree_index = merkle_tree_index[0].clone();
         tmp_account.relayer_fee =  u64::from_le_bytes(fee.try_into().unwrap()).clone();
         tmp_account.recipient = Pubkey::new(&recipient).clone();
         tmp_account.tx_integrity_hash = tx_integrity_hash.clone();
         tmp_account.ext_amount = ext_amount.clone();
         tmp_account.fee = fee.clone();
         tmp_account.leaf_left = leaf_left;
         tmp_account.leaf_right = leaf_right;
         tmp_account.nullifier0 = nullifier0;
         tmp_account.nullifier1 = nullifier1;
         msg!("entering init pairs instruction");
         init_pairs_instruction(tmp_account)?;
         // tmp_account.encrypted_utxos = encrypted_utxos.clone();
         _process_instruction(41,
             tmp_account,
             tmp_account.current_index as usize
         )?;
         tmp_account.current_index =1;
         tmp_account.current_instruction_index = 1;

         // miller loop

         tmp_account.proof_a_bytes = proof[0..64].try_into().unwrap();
         tmp_account.proof_b_bytes = proof[64..64+128].try_into().unwrap();
         tmp_account.proof_c_bytes = proof[64+128..256].try_into().unwrap();
         tmp_account.number_of_steps= 1_350_000; // 1_250_000 compute units for core computation

         tmp_account.f_bytes[0] = 1;
         let proof_b = parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec());

         tmp_account.r_bytes = parse_r_to_bytes(G2HomProjective {
             x: proof_b.x,
             y: proof_b.y,
             z: Fp2::one(),
         });


         // create and initialize
         // let miller_loop_account= &mut ctx.accounts.miller_loop_state.load_init()?;
         // miller_loop_account.signing_address = ctx.accounts.signing_address.key();
         //
         // // miller_loop_account.signing_address = ctx.accounts.signing_address.key();
         // miller_loop_account.prepared_inputs_bytes = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];
         //
         // miller_loop_account.proof_a_bytes = proof[0..64].try_into().unwrap();
         // miller_loop_account.proof_b_bytes = proof[64..64+128].try_into().unwrap();
         // miller_loop_account.proof_c_bytes = proof[64+128..256].try_into().unwrap();
         // miller_loop_account.number_of_steps= 250_000; // 1_250_000 compute units for core computation
         // let mut tmp_account = MillerLoopState::new(
         //     ix_data[224..288].try_into().unwrap(),
         //     ix_data[288..416].try_into().unwrap(),
         //     ix_data[416..480].try_into().unwrap(),
         //     prepared_inputs_bytes.try_into().unwrap(),
         //     1_250_000);
         msg!("finished");
         Ok(())
     }

     pub fn compute(ctx: Context<Compute>, _bump: u64)-> Result<()> {
         let tmp_account= &mut ctx.accounts.verifier_state.load_mut()?;
         msg!("CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}", CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]);
        if tmp_account.current_instruction_index < (IX_ORDER.len() - 1).try_into().unwrap() {
            _process_instruction(IX_ORDER[tmp_account.current_instruction_index as usize],
                tmp_account,
                usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize])
            )?;
            tmp_account.current_index +=1;
        } else {
            msg!("Computing miller_loop");
            // let g_ic_affine =
            //     parse_x_group_affine_from_bytes(&tmp_account.x_1_range); // 10k
            // let p2: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
            //     ark_ec::bn::g1::G1Prepared::from(g_ic_affine);

            let prepared_inputs_expected_res = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];
            assert_eq!(tmp_account.x_1_range.to_vec(),prepared_inputs_expected_res.to_vec(), "prepared inputs failed");
            msg!("computing miller_loop {}", tmp_account.current_instruction_index);
            miller_loop_process_instruction(tmp_account);
        }

         tmp_account.current_instruction_index +=1;
         Ok(())
     }


     pub fn create_miller_loop_account(ctx: Context<CreateMillerLoopState>,
            proof:        [u8;256],
            // prepared_inputs: [u8;64],
     ) -> Result<()> {
         msg!("initializing miller loop account");
         // create and initialize
         let miller_loop_account= &mut ctx.accounts.miller_loop_state.load_init()?;
         miller_loop_account.signing_address = ctx.accounts.signing_address.key();
         miller_loop_account.prepared_inputs_bytes = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];
                 miller_loop_account.proof_a_bytes = proof[0..64].try_into().unwrap();
         miller_loop_account.proof_b_bytes = proof[64..64+128].try_into().unwrap();
         miller_loop_account.proof_c_bytes = proof[64+128..256].try_into().unwrap();
         miller_loop_account.number_of_steps= 1_350_000; // 1_250_000 compute units for core computation

         miller_loop_account.f_bytes[0] = 1;
         let proof_b = parse_proof_b_from_bytes(&miller_loop_account.proof_b_bytes.to_vec());

         miller_loop_account.r_bytes = parse_r_to_bytes(G2HomProjective {
             x: proof_b.x,
             y: proof_b.y,
             z: Fp2::one(),
         });

         // let mut tmp_account = MillerLoopState::new(
         //     ix_data[224..288].try_into().unwrap(),
         //     ix_data[288..416].try_into().unwrap(),
         //     ix_data[416..480].try_into().unwrap(),
         //     prepared_inputs_bytes.try_into().unwrap(),
         //     1_250_000);
         msg!("finished");
         Ok(())
     }
     /*
     pub fn compute_miller_loop(ctx: Context<ComputeMillerLoop>, bump:u8)-> Result<()> {
         // let tmp_account= &mut ctx.accounts.prepare_inputs_state.load_mut();
         // match tmp_account {
         //     Some(tmp_account) => {
         //         msg!("CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}", CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]);
         //         if tmp_account.current_instruction_index == 35 {
         //             let prepared_inputs = ;
         //
         //
         //             miller_loop_account
         //         }
         //         _process_instruction(IX_ORDER[tmp_account.current_instruction_index as usize],
         //             tmp_account,
         //             usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize])
         //         )?;
         //         tmp_account.current_index +=1;
         //         tmp_account.current_instruction_index +=1;
         //     }
         //     _ => {
         //
         //     }
         // }

         let miller_loop_account= &mut ctx.accounts.miller_loop_state.load_mut()?;
         msg!("computing miller_loop {}", miller_loop_account.current_instruction_index);
         miller_loop_process_instruction(miller_loop_account);

         Ok(())
     }

     pub fn create_final_exponentiation_account(ctx: Context<CreateFinalExponentiationState>,
            // miller_loop_bytes:  [u8;384],
            // prepared_inputs: [u8;64],
     ) -> Result<()> {
         msg!("initializing miller loop account");
         // create and initialize
         let final_exponentiation_account= &mut ctx.accounts.final_exponentiation_state.load_init()?;
         final_exponentiation_account.signing_address = ctx.accounts.signing_address.key();
         let miller_loop_bytes = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];

         let f = parse_f_from_bytes(&miller_loop_bytes.to_vec());
         let mut f1 = f.clone();
         f1.conjugate();

        final_exponentiation_account.f = miller_loop_bytes;

        final_exponentiation_account.f1 = parse_f_to_bytes(f1);
        final_exponentiation_account.f2[0] = 1;
        final_exponentiation_account.f3[0] = 1;
        final_exponentiation_account.f4[0] = 1;
        final_exponentiation_account.f5[0] = 1;
        final_exponentiation_account.i[0] = 1;

        final_exponentiation_account.outer_loop = 1;
        final_exponentiation_account.max_compute = 1_200_000;

         // let mut tmp_account = MillerLoopState::new(
         //     ix_data[224..288].try_into().unwrap(),
         //     ix_data[288..416].try_into().unwrap(),
         //     ix_data[416..480].try_into().unwrap(),
         //     prepared_inputs_bytes.try_into().unwrap(),
         //     1_250_000);
         msg!("finished");
         Ok(())
     }

     pub fn compute_final_exponetiation(ctx: Context<ComputeFinalExponentiation>, bump:u64)-> Result<()> {

         let final_exponentiation_account= &mut ctx.accounts.final_exponentiation_state.load_mut()?;
         msg!("computing final_exponentiation {}", final_exponentiation_account.current_instruction_index);
         final_exponentiation_process_instruction(final_exponentiation_account);

         Ok(())
     }
     */
}




#[derive(Accounts)]
pub struct Compute<'info> {
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    pub signing_address: Signer<'info>,
}


#[derive(Accounts)]
pub struct ComputeMillerLoop<'info> {
    #[account(mut)]
    pub miller_loop_state: AccountLoader<'info, MillerLoopState>,
    pub signing_address: Signer<'info>,
}
// signing_address.key().as_ref()
#[derive(Accounts)]
#[instruction(
    proof:[u8;256],
    root_hash:          [u8;32],
    amount:             [u8;32],
    tx_integrity_hash: [u8;32]
)]
pub struct CreateInputsState<'info> {
    #[account(init, seeds = [b"prepare_inputs", tx_integrity_hash.as_ref()], bump, payer=signing_address, space= 3072 as usize)]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateMillerLoopState<'info> {
    #[account(init, seeds = [b"miller_loop", signing_address.key().as_ref()], bump, payer=signing_address, space= 3072 as usize)]
    pub miller_loop_state: AccountLoader<'info, MillerLoopState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateFinalExponentiationState<'info> {
    #[account(init, seeds = [b"final_exponentiation", signing_address.key().as_ref()], bump, payer=signing_address, space= 3048 as usize)]
    pub final_exponentiation_state: AccountLoader<'info, FinalExponentiationState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ComputeFinalExponentiation<'info> {
    #[account(mut)]
    pub final_exponentiation_state: AccountLoader<'info, FinalExponentiationState>,
    pub signing_address: Signer<'info>,
}




#[error_code]
pub enum ErrorCode {
    #[msg("Incompatible Verifying Key")]
    IncompatibleVerifyingKey
}

pub const IX_ORDER: [u8; 37] = [
    //init data happens before this array starts
    //check root
    1, //prepare inputs for verification
    /*40, */ //41,
    42, 42, 42, 42, 62, //46, 41,
    43, 43, 43, 43, 63,
    44, 44, 44, 44, 64,
    45, 45, 45, 45, 65,
    56, 56, 56, 56, 66,
    57, 57, 57, 57, 67,
    58, 58, 58, 58, 68,
    11//miller loop
    /*0, 1, 2, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8,
    4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6,
    8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5,
    6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4,
    5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 10, 4, 5, 6, 11, 4, 5, 6, //final exp
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 10, 11, 14, 15, 15, 15, 15, 16, 17, 15, 15, 16,
    17, 15, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 15, 16, 17, 15,
    15, 16, 17, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 15, 15, 16,
    17, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15,
    15, 16, 17, 15, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 15, 15, 18, 19, 15, 15, 15,
    15, 16, 17, 20, 21, 22, 23, 24, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 28, 29, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29,
    25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25, 25, 25, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 25, 25, 28, 29, 25, 25, 25, 25, 26, 27, 30, 31, 32, 32,
    32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35,
    36, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32,
    33, 34, 32, 32, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35,
    36, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32,
    32, 32, 35, 36, 32, 32, 32, 32, 33, 34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 38, 39, 52, 53, 54, 55, 42, 43, //merkle tree insertion height 18
    34, 14, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3,
    25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3,
    //perform last checks and transfer requested amount
    241,*/
];
