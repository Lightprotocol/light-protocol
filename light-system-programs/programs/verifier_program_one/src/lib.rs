#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_verifier_program_one",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-onchain/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-onchain"
}

pub mod processor;
pub mod verifying_key;

use light_macros::light_verifier_accounts;
pub use processor::*;

use anchor_lang::prelude::*;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::state::VerifierState10Ins;
use merkle_tree_program::program::MerkleTreeProgram;

declare_id!("J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc");

#[constant]
pub const PROGRAM_ID: &str = "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc";

#[program]
pub mod verifier_program_one {
    use light_verifier_sdk::light_transaction::{Amounts, Proof};

    use super::*;

    /// This instruction is the first step of a shielded transaction with 10 inputs and 2 outputs.
    /// It creates and initializes a verifier state account which stores public inputs and other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the verification and
    /// protocol logicin the second transaction.
    pub fn shielded_transfer_first<'info>(
        ctx: Context<'_, '_, '_, 'info, LightInstructionFirst<'info, 0>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: InstructionDataShieldedTransferFirst =
            InstructionDataShieldedTransferFirst::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;
        let proof = Proof {
            a: [0u8; 64],
            b: [0u8; 128],
            c: [0u8; 64],
        };
        let public_amount = Amounts {
            sol: inputs.public_amount_sol,
            spl: inputs.public_amount_spl,
        };
        let len_missing_bytes = 256 - inputs.encrypted_utxos.len();
        let mut enc_utxos = inputs.encrypted_utxos;
        enc_utxos.append(&mut vec![0u8; len_missing_bytes]);
        process_transfer_10_ins_2_outs_first(
            ctx,
            &proof,
            &public_amount,
            &inputs.input_nullifier,
            &[[inputs.output_commitment[0], inputs.output_commitment[1]]; 1],
            &enc_utxos,
            inputs.root_index,
            inputs.relayer_fee,
        )
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn shielded_transfer_second<'info>(
        ctx: Context<'_, '_, '_, 'info, LightInstructionSecond<'info, 0>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: InstructionDataShieldedTransferSecond =
            InstructionDataShieldedTransferSecond::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;
        let proof = Proof {
            a: inputs.proof_a,
            b: inputs.proof_b,
            c: inputs.proof_c,
        };
        process_transfer_10_ins_2_outs_second(ctx, &proof, [0u8; 32])
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'info>(
        _ctx: Context<'_, '_, '_, 'info, CloseVerifierState<'info, 0>>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        seeds = [
            &signing_address.key().to_bytes(),
            VERIFIER_STATE_SEED
        ],
        bump,
        space = 3000 /*8 + 32 * 6 + 10 * 32 + 2 * 32 + 512 + 16 + 128*/,
        payer = signing_address
    )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionConfig>>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataShieldedTransferFirst {
    public_amount_spl: [u8; 32],
    input_nullifier: [[u8; 32]; 10],
    output_commitment: [[u8; 32]; 2],
    public_amount_sol: [u8; 32],
    root_index: u64,
    relayer_fee: u64,
    encrypted_utxos: Vec<u8>,
}

/// Executes light transaction with state created in the first instruction.
#[light_verifier_accounts(sol, spl, signing_address=verifier_state.signer)]
#[derive(Accounts)]
pub struct LightInstructionSecond<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(
        mut,
        seeds = [
            &signing_address.key().to_bytes(),
            VERIFIER_STATE_SEED
        ],
        bump,
        close=signing_address
    )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionConfig>>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataShieldedTransferSecond {
    proof_a: [u8; 64],
    proof_b: [u8; 128],
    proof_c: [u8; 64],
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionConfig>>,
}
