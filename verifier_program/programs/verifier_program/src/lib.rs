pub mod groth16_verifier;
pub mod utils;
use anchor_lang::solana_program::system_program;

// use crate::groth16_verifier::prepare_inputs::state::{
//     CreateVerifierState
// };
use groth16_verifier::prepare_inputs::*;
use groth16_verifier::miller_loop::*;
use crate::groth16_verifier::final_exponentiation_process_instruction;

use ark_ec::bn::g2::G2HomProjective;
use crate::groth16_verifier::parse_r_to_bytes;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use ark_ff::Fp2;
use ark_std::One;
use crate::groth16_verifier::parsers::*;

use anchor_lang::prelude::*;

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
         tmp_account.computing_prepared_inputs = true;

         // miller loop
         tmp_account.proof_a_bytes = proof[0..64].try_into().unwrap();
         tmp_account.proof_b_bytes = proof[64..64+128].try_into().unwrap();
         tmp_account.proof_c_bytes = proof[64+128..256].try_into().unwrap();
         tmp_account.compute_max_miller_loop= 1_350_000; // 1_250_000 compute units for core computation

         tmp_account.f_bytes[0] = 1;
         let proof_b = parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec());

         tmp_account.r_bytes = parse_r_to_bytes(G2HomProjective {
             x: proof_b.x,
             y: proof_b.y,
             z: Fp2::one(),
         });
         msg!("finished");
         Ok(())
     }

     pub fn compute(ctx: Context<Compute>, _bump: u64)-> Result<()> {
         let tmp_account= &mut ctx.accounts.verifier_state.load_mut()?;
        if tmp_account.computing_prepared_inputs /*&& tmp_account.current_instruction_index < (IX_ORDER.len() - 1).try_into().unwrap()*/ {
            msg!("CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}", CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]);
            _process_instruction(IX_ORDER[tmp_account.current_instruction_index as usize],
                tmp_account,
                usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize])
            )?;
            tmp_account.current_index +=1;
        } else if tmp_account.computing_miller_loop {
            tmp_account.max_compute = 1_200_000;

            msg!("computing miller_loop {}", tmp_account.current_instruction_index);
            miller_loop_process_instruction(tmp_account);

        } else {
            if !tmp_account.computing_final_exponentiation {
                msg!("initializing for final_exponentiation");
                tmp_account.computing_final_exponentiation = true;
                msg!("initializing for tmp_account.f_bytes{:?}", tmp_account.f_bytes);
                let mut f1 = parse_f_from_bytes(&tmp_account.f_bytes.to_vec());
                f1.conjugate();
               tmp_account.f_bytes1 = parse_f_to_bytes(f1);
               tmp_account.f_bytes2[0] = 1;
               tmp_account.f_bytes3[0] = 1;
               tmp_account.f_bytes4[0] = 1;
               tmp_account.f_bytes5[0] = 1;
               tmp_account.i_bytes[0] = 1;
               tmp_account.outer_loop = 1;
               tmp_account.max_compute = 1_100_000;
           }

               msg!("computing final_exponentiation {}", tmp_account.current_instruction_index);
               final_exponentiation_process_instruction(tmp_account);



        }

         tmp_account.current_instruction_index +=1;
         Ok(())
     }

}




#[derive(Accounts)]
pub struct Compute<'info> {
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    pub signing_address: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(
    proof:[u8;256],
    root_hash:          [u8;32],
    amount:             [u8;32],
    tx_integrity_hash: [u8;32]
)]
pub struct CreateInputsState<'info> {
    #[account(init, seeds = [b"prepare_inputs", tx_integrity_hash.as_ref()], bump, payer=signing_address, space= 5 * 1024 as usize)]
    pub verifier_state: AccountLoader<'info, VerifierState>,

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
];
