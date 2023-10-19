use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use light_verifier_sdk::state::VerifierState10Ins;
use std::marker::PhantomData;
pub mod psp_accounts;
pub use psp_accounts::*;
pub mod auto_generated_accounts;
pub use auto_generated_accounts::*;
pub mod processor;
pub use processor::*;
pub mod verifying_key_streaming_payments;
pub use verifying_key_streaming_payments::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[constant]
pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

#[program]
pub mod streaming_payments {
    use super::*;
    use solana_program::sysvar;
    /// This instruction is the first step of a shielded transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn light_instruction_first<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionFirst<'info, { VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs }, 4, 4>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs_des: InstructionDataLightInstructionFirst =
            InstructionDataLightInstructionFirst::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;

        let mut program_id_hash = hash(&ctx.program_id.to_bytes()).to_bytes();
        program_id_hash[0] = 0;

        let mut checked_public_inputs: [[u8; 32]; VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs] =
            [[0u8; 32]; VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs];
        checked_public_inputs[0] = program_id_hash;
        checked_public_inputs[1] = inputs_des.transaction_hash;

        let state = VerifierState10Ins {
            merkle_root_index: inputs_des.root_index,
            signer: Pubkey::from([0u8; 32]),
            nullifiers: inputs_des.input_nullifier.to_vec(),
            leaves: inputs_des.output_commitment.to_vec(),
            public_amount_spl: inputs_des.public_amount_spl,
            public_amount_sol: inputs_des.public_amount_sol,
            mint_pubkey: [0u8; 32],
            merkle_root: [0u8; 32],
            tx_integrity_hash: [0u8; 32],
            relayer_fee: inputs_des.relayer_fee,
            encrypted_utxos: inputs_des.encrypted_utxos,
            checked_public_inputs,
            proof_a: [0u8; 64],
            proof_b: [0u8; 128],
            proof_c: [0u8; 64],
            transaction_hash: [0u8; 32],
            e_phantom: PhantomData,
        };

        ctx.accounts.verifier_state.set_inner(state);
        ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;

        Ok(())
    }

    pub fn light_instruction_second<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionSecond<'info, { VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs }, 4, 4>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        inputs.chunks(32).enumerate().for_each(|(i, input)| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(input);
            ctx.accounts.verifier_state.checked_public_inputs[2 + i] = arr
        });
        Ok(())
    }

    /// This instruction is the third step of a shielded transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn light_instruction_third<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionThird<'info, { VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs }, 4, 4>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let current_slot = <Clock as sysvar::Sysvar>::get()?.slot;
        msg!(
            "{} > {}",
            current_slot,
            u64::from_be_bytes(
                ctx.accounts.verifier_state.checked_public_inputs[2][24..32]
                    .try_into()
                    .unwrap(),
            )
        );
        if current_slot
            < u64::from_be_bytes(
                ctx.accounts.verifier_state.checked_public_inputs[2][24..32]
                    .try_into()
                    .unwrap(),
            )
        {
            panic!("Escrow still locked");
        }
        msg!(
            "checked inputs {:?}",
            ctx.accounts.verifier_state.checked_public_inputs
        );
        verify_program_proof(&ctx, &inputs)?;
        cpi_verifier_two(&ctx, &inputs)
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<
            'a,
            'b,
            'c,
            'info,
            CloseVerifierState<'info, { VERIFYINGKEY_STREAMING_PAYMENTS.nr_pubinputs }, 4, 4>,
        >,
    ) -> Result<()> {
        Ok(())
    }
}
