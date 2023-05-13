/*
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_market_place_verifier",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}
*/
pub mod light_utils;
// pub use light_utils;
pub mod processor;
pub mod verifying_key;
use crate::light_utils::*;

use crate::processor::{
    process_transfer_4_ins_4_outs_4_checked_first, process_transfer_4_ins_4_outs_4_checked_third,
};
use anchor_lang::prelude::*;
pub use processor::*;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[constant]
pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

#[program]
pub mod mock_verifier {
    use anchor_lang::solana_program::keccak::hash;

    use super::*;

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn light_instruction_first<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs_des: InstructionDataLightInstructionFirst =
            InstructionDataLightInstructionFirst::try_deserialize_unchecked(
                &mut inputs.as_slice(),
            )?;
        let proof_a = [0u8; 64];
        let proof_b = [0u8; 128];
        let proof_c = [0u8; 64];
        let pool_type = [0u8; 32];
        let checked_inputs = vec![
            [
                vec![0u8],
                hash(&ctx.program_id.to_bytes()).try_to_vec()?[1..].to_vec(),
            ]
            .concat(),
            inputs_des.transaction_hash.to_vec(),
            // inputs_des.current_slot.to_vec(),
        ];
        process_transfer_4_ins_4_outs_4_checked_first(
            ctx,
            &proof_a,
            &proof_b,
            &proof_c,
            &inputs_des.public_amount_spl,
            &inputs_des.input_nullifier,
            &inputs_des.output_commitment,
            &inputs_des.public_amount_sol,
            &checked_inputs,
            &inputs_des.encrypted_utxos,
            &pool_type,
            &inputs_des.root_index,
            &inputs_des.relayer_fee,
        )
    }

    pub fn light_instruction_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        // cut off discriminator
        let vec = &inputs[8..];
        let _ = vec
            .chunks(32)
            .map(|input| {
                ctx.accounts
                    .verifier_state
                    .checked_public_inputs
                    .push(input.to_vec())
            })
            .collect::<Vec<_>>();
        Ok(())
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn light_instruction_third<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionThird<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs_des: InstructionDataLightInstructionThird =
            InstructionDataLightInstructionThird::try_deserialize(&mut inputs.as_slice())?;

        process_transfer_4_ins_4_outs_4_checked_third(
            ctx,
            &inputs_des.proof_a_app,
            &inputs_des.proof_b_app,
            &inputs_des.proof_c_app,
            &inputs_des.proof_a,
            &inputs_des.proof_b,
            &inputs_des.proof_c,
        )
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, CloseVerifierState<'info>>,
    ) -> Result<()> {
        Ok(())
    }
}
