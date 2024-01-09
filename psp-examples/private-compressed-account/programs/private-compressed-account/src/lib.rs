use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
pub mod psp_accounts;
pub use psp_accounts::*;
pub mod processor;
pub use processor::*;
pub mod verifying_key_compressed_account_update;
pub use verifying_key_compressed_account_update::*;
pub mod verifying_key_inclusion_proof;
pub use verifying_key_inclusion_proof::*;
use light_psp4in4out_app_storage::Psp4In4OutAppStorageVerifierState;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[constant]
pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

#[program]
pub mod private_compressed_account {
    use light_verifier_sdk::light_transaction::Proof;

    use super::*;

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
            LightInstructionFirst<
                'info,
                { VERIFYINGKEY_COMPRESSED_ACCOUNT_UPDATE.nr_pubinputs },                
            >,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs_des: InstructionDataLightInstructionFirst =
            InstructionDataLightInstructionFirst::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;

        let mut program_id_hash = hash(&ctx.program_id.to_bytes()).to_bytes();
        program_id_hash[0] = 0;

        let mut verifier_state = ctx.accounts.verifier_state.load_init()?;
        verifier_state.signer = *ctx.accounts.signing_address.key;
        let verifier_state_data = Psp4In4OutAppStorageVerifierState {
            nullifiers: inputs_des.input_nullifier,
            leaves: inputs_des.output_commitment.try_into().unwrap(),
            public_amount_spl: inputs_des.public_amount_spl,
            public_amount_sol: inputs_des.public_amount_sol,
            rpc_fee: inputs_des.rpc_fee,
            encrypted_utxos: inputs_des.encrypted_utxos.try_into().unwrap(),
            merkle_root_index: inputs_des.root_index,
        };
        let mut verifier_state_vec = Vec::new();
        Psp4In4OutAppStorageVerifierState::serialize(&verifier_state_data, &mut verifier_state_vec)
            .unwrap();
        verifier_state.verifier_state_data = [verifier_state_vec, vec![0u8; 1024 - 848]]
            .concat()
            .try_into()
            .unwrap();

        verifier_state.checked_public_inputs[0] = program_id_hash;
        verifier_state.checked_public_inputs[1] = inputs_des.transaction_hash;

        Ok(())
    }

    pub fn light_instruction_compressed_account_update_second<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionSecond<
                'info,
                { VERIFYINGKEY_COMPRESSED_ACCOUNT_UPDATE.nr_pubinputs },
            >,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut verifier_state = ctx.accounts.verifier_state.load_mut()?;
        inputs.chunks(32).enumerate().for_each(|(i, input)| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(input);
            verifier_state.checked_public_inputs[i] = arr
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
            LightInstructionThird<
                'info,
                { VERIFYINGKEY_COMPRESSED_ACCOUNT_UPDATE.nr_pubinputs },                
            >,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let verifier_state = ctx.accounts.verifier_state.load()?;
        let mut compressed_account_merkle_tree =
            ctx.accounts.compressed_account_merkle_tree.load_mut()?;

        if compressed_account_merkle_tree.sub_tree_hash
            != verifier_state.checked_public_inputs[2]
        {
            // return Err(ErrorCode::InvalidSubTreeHash.into());
            msg!(
                "checked inputs {:?}",
                verifier_state.checked_public_inputs
            );
            msg!(
                "sub tree hash {:?}",
                compressed_account_merkle_tree.sub_tree_hash
            );
            panic!("InvalidSubTreeHash");
        }
        let mut checked_public_inputs = verifier_state.checked_public_inputs[2];
        checked_public_inputs = compressed_account_merkle_tree.sub_tree_hash;
        verify_program_proof(&ctx, &inputs)?;
        cpi_verifier_two(&ctx, &inputs)?;
        let current_root_index = compressed_account_merkle_tree.current_root_index;
        compressed_account_merkle_tree.next_leaf_index += 1;
        // inserting new root
        compressed_account_merkle_tree.root_history[current_root_index as usize] =
        verifier_state.checked_public_inputs[0];
        // inserting new sub tree hash
        compressed_account_merkle_tree.sub_tree_hash =
        verifier_state.checked_public_inputs[3];
        compressed_account_merkle_tree.current_root_index =
            (compressed_account_merkle_tree.current_root_index + 1) % ROOT_HISTORY_SIZE as u64;
        Ok(())
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<
            'a,
            'b,
            'c,
            'info,
            CloseVerifierState<
                'info,
                { VERIFYINGKEY_COMPRESSED_ACCOUNT_UPDATE.nr_pubinputs },                
            >,
        >,
    ) -> Result<()> {
        Ok(())
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn init_compressed_account_merkle_tree<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitCompressionMerkleTree<'info>>,
        _index: u64,
    ) -> Result<()> {
        let mut compressed_account_merkle_tree =
            ctx.accounts.compressed_account_merkle_tree.load_init()?;
        compressed_account_merkle_tree.sub_tree_hash = ZERO_VALUES_SUB_TREE_HASH;
        Ok(())
    }

    pub fn prove_inclusion_value_gte(
        ctx: Context<ProveInclusionInstruction>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        root_index: u64,
        value: u64,
    ) -> Result<()> {
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
        let compressed_account_merkle_tree = ctx.accounts.compressed_account_merkle_tree.load()?;

        verify_inclusion_proof(
            &proof,
            compressed_account_merkle_tree.root_history[root_index as usize],
            [vec![0u8; 24], value.to_be_bytes().to_vec()]
                .concat()
                .try_into()
                .unwrap(),
        )?;
        Ok(())
    }
}
