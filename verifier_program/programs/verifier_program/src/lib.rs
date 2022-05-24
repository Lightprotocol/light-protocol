pub mod groth16_verifier;
pub mod utils;
use anchor_lang::solana_program::system_program;

// use crate::groth16_verifier::prepare_inputs::state::{
//     CreatePrepareInputsState
// };
use groth16_verifier::prepare_inputs::*;
use groth16_verifier::miller_loop::*;

use ark_ec::bn::g2::G2HomProjective;
use crate::groth16_verifier::parse_r_to_bytes;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use ark_ff::Fp2;
use ark_std::One;

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


         let tmp_account= &mut ctx.accounts.prepare_inputs_state.load_init()?;
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
         tmp_account.proof_a_b_c = proof.clone();
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

         // create and initialize
         let miller_loop_account= &mut ctx.accounts.miller_loop_state.load_init()?;
         miller_loop_account.signing_address = ctx.accounts.signing_address.key();

         // miller_loop_account.signing_address = ctx.accounts.signing_address.key();
         miller_loop_account.prepared_inputs_bytes = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];

         miller_loop_account.proof_a_bytes = proof[0..64].try_into().unwrap();
         miller_loop_account.proof_b_bytes = proof[64..64+128].try_into().unwrap();
         miller_loop_account.proof_c_bytes = proof[64+128..256].try_into().unwrap();
         miller_loop_account.number_of_steps= 1_250_000; // 1_250_000 compute units for core computation
         // let mut tmp_account = MillerLoopState::new(
         //     ix_data[224..288].try_into().unwrap(),
         //     ix_data[288..416].try_into().unwrap(),
         //     ix_data[416..480].try_into().unwrap(),
         //     prepared_inputs_bytes.try_into().unwrap(),
         //     1_250_000);
         msg!("finished");
         Ok(())
     }

     pub fn prepare_inputs(ctx: Context<PrepareInputs>)-> Result<()> {
         let tmp_account= &mut ctx.accounts.prepare_inputs_state.load_mut()?;
         msg!("CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}", CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]);

         _process_instruction(IX_ORDER[tmp_account.current_instruction_index as usize],
             tmp_account,
             usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize])
         )?;
         tmp_account.current_index +=1;
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
}

#[derive(Accounts)]
pub struct ComputeMillerLoop<'info> {
    #[account(mut)]
    pub miller_loop_state: AccountLoader<'info, MillerLoopState>,
    pub signing_address: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateInputsState<'info> {
    #[account(init, seeds = [b"prepare_inputs", signing_address.key().as_ref()], bump, payer=signing_address, space= 2048 as usize)]
    pub prepare_inputs_state: AccountLoader<'info, PrepareInputsState>,

    #[account(init, seeds = [b"miller_loop", signing_address.key().as_ref()], bump, payer=signing_address, space= 2048 as usize)]
    pub miller_loop_state: AccountLoader<'info, MillerLoopState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateMillerLoopState<'info> {
    #[account(init, seeds = [b"miller_loop", signing_address.key().as_ref()], bump, payer=signing_address, space= 2048 as usize)]
    pub miller_loop_state: AccountLoader<'info, MillerLoopState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
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
