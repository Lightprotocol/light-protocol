pub mod groth16_verifier;
pub mod instructions_last_transaction;
pub mod processor_last_transaction;
pub mod state;
pub mod utils;
pub use instructions_last_transaction::*;

use crate::state::VerifierState;
use crate::groth16_verifier::{
    prepare_inputs::*,
    final_exponentiation_process_instruction,
    miller_loop::*,
    parsers::*,
};

use ark_ec::bn::g2::G2HomProjective;
use ark_ff::Fp2;
use ark_std::One;

use anchor_lang::prelude::*;
use merkle_tree_program::{
    self,
    program::MerkleTreeProgram,
    utils::config::STORAGE_SEED,
    wrapped_state:: {MerkleTree},
};
use merkle_tree_program::instructions::sol_transfer;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
    use super::*;

    // Creates and initializes a state account to save state of a verification for one transaction
    pub fn create_verifier_state(
        ctx: Context<CreateVerifierState>,
        proof: [u8; 256],
        root_hash: [u8; 32],
        amount: [u8; 32],
        tx_integrity_hash: [u8; 32],
        nullifier0: [u8; 32],
        nullifier1: [u8; 32],
        leaf_right: [u8; 32],
        leaf_left: [u8; 32],
        recipient: [u8; 32],
        ext_amount: [u8; 8],
        _relayer: [u8; 32],
        relayer_fee: [u8; 8],
        encrypted_utxos: [u8; 256],
        merkle_tree_index: [u8; 1],
    ) -> Result<()> {
        // if not initialized this will run load_init
        let tmp_account = &mut match ctx.accounts.verifier_state.load_mut() {
                Ok(res) => res,
                Err(_)  => ctx.accounts.verifier_state.load_init()?
        };

        tmp_account.signing_address = ctx.accounts.signing_address.key();
        tmp_account.root_hash = root_hash.clone();
        tmp_account.amount = amount.clone();
        tmp_account.merkle_tree_index = merkle_tree_index[0].clone();
        tmp_account.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();
        tmp_account.recipient = Pubkey::new(&recipient).clone();
        tmp_account.tx_integrity_hash = tx_integrity_hash.clone();
        tmp_account.ext_amount = ext_amount.clone();
        tmp_account.fee = relayer_fee.clone();//tx_fee.clone();
        tmp_account.leaf_left = leaf_left;
        tmp_account.leaf_right = leaf_right;
        tmp_account.nullifier0 = nullifier0;
        tmp_account.nullifier1 = nullifier1;
        tmp_account.encrypted_utxos = encrypted_utxos[..222].try_into().unwrap();

        // initing pairs to prepared inputs
        init_pairs_instruction(tmp_account)?;
        _process_instruction(41, tmp_account, tmp_account.current_index as usize)?;
        tmp_account.current_index = 1;
        tmp_account.current_instruction_index = 1;
        tmp_account.computing_prepared_inputs = true;

        // miller loop
        tmp_account.proof_a_bytes = proof[0..64].try_into().unwrap();
        tmp_account.proof_b_bytes = proof[64..64 + 128].try_into().unwrap();
        tmp_account.proof_c_bytes = proof[64 + 128..256].try_into().unwrap();
        tmp_account.ml_max_compute = 1_350_000;
        tmp_account.f_bytes[0] = 1;
        let proof_b = parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec());

        tmp_account.r_bytes = parse_r_to_bytes(G2HomProjective {
            x: proof_b.x,
            y: proof_b.y,
            z: Fp2::one(),
        });


        Ok(())
    }

    pub fn create_escrow_state(
        ctx: Context<CreateEscrowState>,
        tx_integrity_hash: [u8; 32],
        tx_fee: u64,
        relayer_fee: [u8;8],
        amount: u64
    )-> Result<()> {
        msg!("starting initializing escrow account");

        // init escrow account
        let fee_escrow_state = &mut ctx.accounts.fee_escrow_state;

        fee_escrow_state.verifier_state_pubkey = ctx.accounts.verifier_state.key();
        fee_escrow_state.relayer_pubkey = ctx.accounts.signing_address.key();
        fee_escrow_state.user_pubkey = ctx.accounts.user.key();
        fee_escrow_state.tx_fee = tx_fee;//u64::from_le_bytes(tx_fee.try_into().unwrap()).clone();// fees for tx (tx_fee = number_of_tx * 0.000005)
        fee_escrow_state.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();// for relayer
        // fee_escrow_state.creation_slot = anchor_lang::prelude::Clock::slot;
        // sol_transfer(
        //     &ctx.accounts.user.to_account_info(),
        //     &ctx.accounts.signing_address.to_account_info(),
        //     amount
        // );
        let cpi_ctx1 = CpiContext::new(ctx.accounts.system_program.to_account_info(), anchor_lang::system_program::Transfer{
             from: ctx.accounts.user.to_account_info(),
             to: ctx.accounts.fee_escrow_state.to_account_info()
         });
        anchor_lang::system_program::transfer(cpi_ctx1, amount)?;
        msg!(" initialized escrow account");
        Ok(())
    }

    // Creates and initializes a merkle tree state account to save state of hash computations during the Merkle tree update
    /*pub fn create_merkle_tree_update_state(ctx: Context<CreateMerkleTreeUpdateState>) -> Result<()> {
        let tmp_account = &mut ctx.accounts.verifier_state.load()?;

        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts = merkle_tree_program::cpi::accounts::InitializeMerkleTreeUpdateState {
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
        ]
        .concat();

        let cpi_ctx = CpiContext::new(merkle_tree_program_id, accounts);
        merkle_tree_program::cpi::initialize_merkle_tree_update_state(cpi_ctx, data).unwrap();
        Ok(())
    }
    */

    // Verifies Groth16 ZKPs and updates the Merkle tree
    pub fn compute(ctx: Context<Compute>, _bump: u64) -> Result<()> {
        let tmp_account = &mut ctx.accounts.verifier_state.load_mut()?;

        if tmp_account.computing_prepared_inputs
        {
            msg!(
                "CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}",
                CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]
            );
            _process_instruction(
                IX_ORDER[tmp_account.current_instruction_index as usize],
                tmp_account,
                usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]),
            )?;
            tmp_account.current_index += 1;
        } else if tmp_account.computing_miller_loop {
            tmp_account.ml_max_compute = 1_300_000;

            msg!(
                "computing miller_loop {}",
                tmp_account.current_instruction_index
            );
            miller_loop_process_instruction(tmp_account);
        }/* else if tmp_account.updating_merkle_tree {
            let derived_pubkey = Pubkey::find_program_address(
                &[
                    tmp_account.tx_integrity_hash.as_ref(),
                    STORAGE_SEED.as_ref(),
                ],
                ctx.program_id,
            );

            let data = _bump.to_le_bytes().to_vec();

            let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
            let accounts = merkle_tree_program::cpi::accounts::UpdateMerkleTree {
                authority: ctx.accounts.signing_address.to_account_info(),
                merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_state.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            };
            let bump_seed = &[derived_pubkey.1];
            let seeds = [&[
                tmp_account.tx_integrity_hash.as_ref(),
                &b"storage"[..],
                bump_seed,
            ][..]];
            let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
            merkle_tree_program::cpi::update_merkle_tree(cpi_ctx, data)?;
            tmp_account.merkle_tree_instruction_index += 1;
            msg!(
                "merkle_tree_instruction_index {:?}",
                tmp_account.merkle_tree_instruction_index
            );

            if tmp_account.merkle_tree_instruction_index == 74 {
                tmp_account.last_transaction = true;
                tmp_account.updating_merkle_tree = false;
            }
        } */else {
            if !tmp_account.computing_final_exponentiation {
                msg!("Initializing for final_exponentiation.");
                tmp_account.computing_final_exponentiation = true;
                let mut f1 = parse_f_from_bytes(&tmp_account.f_bytes.to_vec());
                f1.conjugate();
                tmp_account.f_bytes1 = parse_f_to_bytes(f1);
                // Initializing temporary storage for final_exponentiation
                // with fqk::zero() which is equivalent to [[1], [0;383]].concat()
                tmp_account.f_bytes2[0] = 1;
                tmp_account.f_bytes3[0] = 1;
                tmp_account.f_bytes4[0] = 1;
                tmp_account.f_bytes5[0] = 1;
                tmp_account.i_bytes[0] = 1;
                // Skipping the first loop iteration since the naf_vec is zero.
                tmp_account.outer_loop = 1;
                // Adjusting max compute limite to 1.2m, we still need some buffer
                // for overhead and varying compute costs depending on the numbers.
                tmp_account.fe_max_compute = 1_200_000;
                // Adding compute costs for packing the initialized fs.
                tmp_account.current_compute+=150_000;
            }

            msg!("Computing final_exponentiation");
            final_exponentiation_process_instruction(tmp_account);
        }

        tmp_account.current_instruction_index += 1;
        Ok(())
    }

    // Transfers the deposit amount,
    // inserts nullifiers and Merkle tree leaves
    pub fn last_transaction_deposit(ctx: Context<LastTransactionDeposit>) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.signing_address.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(merkle_tree_program_id.clone(), accounts);
        merkle_tree_program::cpi::initialize_nullifier(
            cpi_ctx,
            ctx.accounts.verifier_state.load()?.nullifier0
        ).unwrap();

        let merkle_tree_program_id1 = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts1 = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.signing_address.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier1_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx1 = CpiContext::new(merkle_tree_program_id1, accounts1);
        merkle_tree_program::cpi::initialize_nullifier(
            cpi_ctx1,
            ctx.accounts.verifier_state.load()?.nullifier1
        ).unwrap();

        processor_last_transaction::process_last_transaction_deposit(ctx)
    }

    // Transfers the withdrawal amount, pays the relayer,
    // inserts nullifiers and Merkle tree leaves
    pub fn last_transaction_withdrawal(ctx: Context<LastTransactionWithdrawal>) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.signing_address.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(merkle_tree_program_id.clone(), accounts);
        merkle_tree_program::cpi::initialize_nullifier(
            cpi_ctx,
            ctx.accounts.verifier_state.load()?.nullifier0
        ).unwrap();

        let merkle_tree_program_id1 = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts1 = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.signing_address.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier1_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx1 = CpiContext::new(merkle_tree_program_id1, accounts1);
        merkle_tree_program::cpi::initialize_nullifier(
            cpi_ctx1,
            ctx.accounts.verifier_state.load()?.nullifier1
        ).unwrap();

        processor_last_transaction::process_last_transaction_withdrawal(ctx)
    }

    pub fn close_fee_escrow_pda(ctx: Context<CloseFeeEscrowPda>) -> Result<()> {
        let fee_escrow_state = &mut ctx.accounts.fee_escrow_state;
        let verifier_state = &mut ctx.accounts.verifier_state.load()?;
        // this might be unsafe maybe the check doesn't matter anyway because for a withdrawal this
        // account does not exist
        let external_amount: i64 = i64::from_le_bytes(verifier_state.ext_amount);
        // escrow is only applied for deposits
        if external_amount < 0 {
            return err!(ErrorCode::NotDeposit);
        }

        // transfer remaining funds after subtracting the fee
        // for the number of executed transactions to the user
        // TODO make fee per transaction configurable
        let transfer_amount_relayer = verifier_state.current_instruction_index * 50_000;
        msg!("transfer_amount_relayer: {}", transfer_amount_relayer);
        sol_transfer(
            &fee_escrow_state.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            transfer_amount_relayer.try_into().unwrap()

        )?;


        // transfer remaining funds after subtracting the fee
        // for the number of executed transactions to the user
        let transfer_amount_user: u64 =
            fee_escrow_state.relayer_fee
            + fee_escrow_state.tx_fee
            - transfer_amount_relayer as u64
            + external_amount as u64;

        msg!("transfer_amount_user: {}", transfer_amount_user);
        sol_transfer(
            &fee_escrow_state.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            transfer_amount_user.try_into().unwrap()

        )?;
        Ok(())

    }
}

#[derive(Accounts)]
pub struct Compute<'info> {
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    // pub verifier_state_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    pub merkle_tree_tmp_state: UncheckedAccount<'info>,

    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    #[account(mut)]
    pub merkle_tree: Account<'info, MerkleTree>,
}

#[derive(Accounts)]
#[instruction(
    proof:[u8;256],
    root_hash:          [u8;32],
    amount:             [u8;32],
    tx_integrity_hash: [u8;32]
)]
pub struct CreateVerifierState<'info> {
    #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024 as usize)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(
    tx_integrity_hash: [u8;32]
)]
pub struct CreateEscrowState<'info> {
    #[account(init,seeds = [tx_integrity_hash.as_ref(), b"fee_escrow"], bump,  payer=signing_address, space= 256 as usize)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024 as usize)]
    /// CHECK: is ininitialized at this point the
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
/*
#[derive(Accounts)]
pub struct CreateMerkleTreeUpdateState<'info> {
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_tmp_state: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    pub rent: Sysvar<'info, Rent>,
}
*/
#[derive(Accounts)]
pub struct CloseFeeEscrowPda<'info> {
    #[account(mut, close = relayer)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(mut, close = relayer)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut, constraint= user.key() == fee_escrow_state.user_pubkey)]
    /// either user address or relayer address depending on who claims
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub user: AccountInfo<'info>,
    #[account(mut, constraint=relayer.key() == fee_escrow_state.relayer_pubkey )]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub relayer: AccountInfo<'info>,

}

#[account]
pub struct FeeEscrowState {
    pub verifier_state_pubkey: Pubkey,
    pub relayer_pubkey: Pubkey,
    pub user_pubkey:    Pubkey,
    pub tx_fee:        u64,// fees for tx (tx_fee = number_of_tx * 0.000005)
    pub relayer_fee:   u64,// for relayer
    pub creation_slot:  u64
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
    NotLastTransactionState,
    #[msg("Tx is not a deposit")]
    NotDeposit
}
