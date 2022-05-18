pub mod groth16_verifier;
pub mod utils;

// use crate::groth16_verifier::prepare_inputs::state::{
//     CreatePrepareInputsState
// };
use groth16_verifier::prepare_inputs::*;

use anchor_lang::prelude::*;
use solana_program::log::sol_log_compute_units;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
     use super::*;

     pub fn create_tmp_account(ctx: Context<CreatePrepareInputsState>,
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
            merkle_tree_index:  [u8;1],
     ) -> Result<()> {
         // msg!("data{:?}", data);
         // msg!("data1 {:?}", data1);

         let tmp_account= &mut ctx.accounts.prepare_inputs_state.load_init()?;
         tmp_account.signing_address = ctx.accounts.signing_address.key();
         // let x: u8 = tmp_account;
         tmp_account.root_hash = root_hash.clone();
         msg!("root_hash {:?}", tmp_account.root_hash);
         assert_eq!(tmp_account.root_hash, root_hash);
         tmp_account.amount = amount.clone();
         //
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

         sol_log_compute_units();
         msg!("finished");
         Ok(())
     }

     pub fn prepare_inputs(ctx: Context<PrepareInputs>)-> Result<()> {

         Ok(())
     }
}
