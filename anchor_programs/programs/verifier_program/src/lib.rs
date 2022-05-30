pub mod groth16_verifier;
pub mod utils;
pub mod processor_last_transaction;
pub mod state;
pub mod instructions_last_transaction;
pub use instructions_last_transaction::*;

use crate::state::VerifierState;


use crate::groth16_verifier::prepare_inputs::IX_ORDER;

use groth16_verifier::prepare_inputs::*;
use groth16_verifier::miller_loop::*;
use crate::groth16_verifier::final_exponentiation_process_instruction;

use ark_ec::bn::g2::G2HomProjective;
use crate::groth16_verifier::parse_r_to_bytes;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use ark_ff::Fp2;
use ark_std::One;
use crate::groth16_verifier::parsers::*;
use crate::utils::config::STORAGE_SEED;


use anchor_lang::prelude::*;

use merkle_tree_program::{self};

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
            recipient:          [u8;32],
            ext_amount:         [u8;8],
            _relayer:            [u8;32],
            fee:                [u8;8],
            encrypted_utxos:    [u8;256],
            merkle_tree_index:  [u8;1],
     ) -> Result<()> {


         let tmp_account= &mut ctx.accounts.verifier_state.load_init()?;
         tmp_account.signing_address = ctx.accounts.signing_address.key();
         // let x: u8 = tmp_account;
         tmp_account.root_hash = root_hash.clone();
         msg!("root_hash {:?}", tmp_account.root_hash);
         assert_eq!(tmp_account.root_hash, root_hash);
         tmp_account.amount = amount.clone();
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
         tmp_account.encrypted_utxos = encrypted_utxos[..222].try_into().unwrap();
         msg!("tmp_account.encrypted_utxos {:?}", tmp_account.encrypted_utxos);
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



         Ok(())
     }

     pub fn create_merkle_tree_tmp_account(ctx: Context<CreateMerkleTreeState>
     ) -> Result<()> {
         // Can I init this account with data dynamically before the update merkle tree
         // instructions.
         msg!("starting cpi");
         let tmp_account= &mut ctx.accounts.verifier_state.load()?;

         let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
         let accounts = merkle_tree_program::cpi::accounts::InitializeTmpMerkleTree {
             authority: ctx.accounts.signing_address.to_account_info(),
             merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_state.to_account_info(),
             system_program: ctx.accounts.system_program.to_account_info(),
             rent: ctx.accounts.rent.to_account_info(),
         };

         let data = [
             tmp_account.tx_integrity_hash.to_vec(),
             tmp_account.leaf_left.to_vec(),
             tmp_account.leaf_right.to_vec(),
             tmp_account.root_hash.to_vec(),
         ].concat();
         msg!("data: {:?}", data);

         let cpi_ctx = CpiContext::new(merkle_tree_program_id, accounts);
         merkle_tree_program::cpi::initialize_tmp_merkle_tree_state(cpi_ctx, data).unwrap();
         msg!("finished cpi");
         Ok(())
    }

     pub fn compute(ctx: Context<Compute>, _bump: u64)-> Result<()> {
         let tmp_account= &mut ctx.accounts.verifier_state.load_mut()?;
        // tmp_account.computing_prepared_inputs = false;
        // tmp_account.computing_miller_loop= false;
        // tmp_account.updating_merkle_tree = true;
        if tmp_account.computing_prepared_inputs /*&& tmp_account.current_instruction_index < (IX_ORDER.len() - 1).try_into().unwrap()*/ {
            msg!("CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}", CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]);
            _process_instruction(IX_ORDER[tmp_account.current_instruction_index as usize],
                tmp_account,
                usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize])
            )?;
            tmp_account.current_index +=1;
        } else if tmp_account.computing_miller_loop {
            tmp_account.max_compute = 1_300_000;

            msg!("computing miller_loop {}", tmp_account.current_instruction_index);
            miller_loop_process_instruction(tmp_account);

        }
        else if tmp_account.updating_merkle_tree {

            let derived_pubkey =
                 Pubkey::find_program_address(&[tmp_account.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()], ctx.program_id);

            let data = _bump.to_le_bytes().to_vec();

            let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
            let accounts = merkle_tree_program::cpi::accounts::UpdateMerkleTree {
                authority: ctx.accounts.signing_address.to_account_info(),
                merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_state.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            };
            let bump_seed = &[derived_pubkey.1];
            let seeds = [&[tmp_account.tx_integrity_hash.as_ref(), &b"storage"[..], bump_seed][..]];
            msg!("starting cpi updating merkle tree");
            let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
            // let cpi_ctx = CpiContext::new(merkle_tree_program_id, accounts);
            merkle_tree_program::cpi::update_merkle_tree(cpi_ctx, data)?;
            msg!("finished cpi");
            tmp_account.merkle_tree_instruction_index+=1;
            msg!("merkle_tree_instruction_index {:?}", tmp_account.merkle_tree_instruction_index);

            if tmp_account.merkle_tree_instruction_index == 74 {
                tmp_account.last_transaction = true;
                tmp_account.updating_merkle_tree = false;
            }
        } else{

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

               msg!("computing final_exponentiation");
               final_exponentiation_process_instruction(tmp_account);

        }

         tmp_account.current_instruction_index +=1;
         Ok(())
     }


     pub fn last_transaction_deposit(
         ctx: Context<LastTransactionDeposit>,
         nullifier0: [u8;32],
         nullifier1: [u8;32]
     )-> Result<()> {
         let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
         let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
             authority: ctx.accounts.signing_address.to_account_info(),
             nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
             system_program: ctx.accounts.system_program.to_account_info(),
             rent: ctx.accounts.rent.to_account_info(),
         };

         let cpi_ctx = CpiContext::new(merkle_tree_program_id.clone(), accounts);
         merkle_tree_program::cpi::initialize_nullifier(cpi_ctx, nullifier0).unwrap();

         let merkle_tree_program_id1 = ctx.accounts.program_merkle_tree.to_account_info();
         let accounts1 = merkle_tree_program::cpi::accounts::InitializeNullifier {
             authority: ctx.accounts.signing_address.to_account_info(),
             nullifier_pda: ctx.accounts.nullifier1_pda.to_account_info(),
             system_program: ctx.accounts.system_program.to_account_info(),
             rent: ctx.accounts.rent.to_account_info(),
         };

         let cpi_ctx1 = CpiContext::new(merkle_tree_program_id1, accounts1);
         merkle_tree_program::cpi::initialize_nullifier(cpi_ctx1, nullifier1).unwrap();

         processor_last_transaction::process_last_transaction_deposit(ctx)
     }

     pub fn last_transaction_withdrawal(
         ctx: Context<LastTransactionWithdrawal>,
         nullifier0: [u8;32],
         nullifier1: [u8;32]
     )-> Result<()> {
         let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
         let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
             authority: ctx.accounts.signing_address.to_account_info(),
             nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
             system_program: ctx.accounts.system_program.to_account_info(),
             rent: ctx.accounts.rent.to_account_info(),
         };

         let cpi_ctx = CpiContext::new(merkle_tree_program_id.clone(), accounts);
         merkle_tree_program::cpi::initialize_nullifier(cpi_ctx, nullifier0).unwrap();

         let merkle_tree_program_id1 = ctx.accounts.program_merkle_tree.to_account_info();
         let accounts1 = merkle_tree_program::cpi::accounts::InitializeNullifier {
             authority: ctx.accounts.signing_address.to_account_info(),
             nullifier_pda: ctx.accounts.nullifier1_pda.to_account_info(),
             system_program: ctx.accounts.system_program.to_account_info(),
             rent: ctx.accounts.rent.to_account_info(),
         };

         let cpi_ctx1 = CpiContext::new(merkle_tree_program_id1, accounts1);
         merkle_tree_program::cpi::initialize_nullifier(cpi_ctx1, nullifier1).unwrap();

         processor_last_transaction::process_last_transaction_withdrawal(ctx)
     }

}




#[derive(Accounts)]
pub struct Compute<'info> {
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    // pub verifier_state_authority: UncheckedAccount<'info>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_tmp_state: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub program_merkle_tree: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(
    proof:[u8;256],
    root_hash:          [u8;32],
    amount:             [u8;32],
    tx_integrity_hash: [u8;32]
)]
pub struct CreateInputsState<'info> {
    #[account(init, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024 as usize)]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    #[account(mut)]
    pub signing_address: Signer<'info>,
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}


#[derive(Accounts)]
// #[instruction(tx_integrity_hash:  [u8;32])]
pub struct CreateMerkleTreeState<'info> {
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_tmp_state: AccountInfo<'info>,

        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub program_merkle_tree: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub rent: AccountInfo<'info>,

}


#[error_code]
pub enum ErrorCode {
    #[msg("Incompatible Verifying Key")]
    IncompatibleVerifyingKey,
    #[msg("WrongPubAmount")]
    WrongPubAmount,
    #[msg("PrepareInputsDidNotFinish")]
    PrepareInputsDidNotFinish,
    #[msg("NotLastTransactionState")]
    NotLastTransactionState
}
