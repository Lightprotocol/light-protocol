use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
pub mod psp_accounts;
pub use psp_accounts::*;
pub mod auto_generated_accounts;
pub use auto_generated_accounts::*;
pub mod processor;
pub use processor::*;
pub mod verifying_key_rock_paper_scissors;
use light_psp4in4out_app_storage::Psp4In4OutAppStorageVerifierState;
pub use verifying_key_rock_paper_scissors::*;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[constant]
pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

#[program]
pub mod rock_paper_scissors {
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
            LightInstructionFirst<'info, { VERIFYINGKEY_ROCK_PAPER_SCISSORS.nr_pubinputs }>,
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
            relayer_fee: inputs_des.relayer_fee,
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

    pub fn light_instruction_second<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionSecond<'info, { VERIFYINGKEY_ROCK_PAPER_SCISSORS.nr_pubinputs }>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut verifier_state = ctx.accounts.verifier_state.load_mut()?;
        inputs.chunks(32).enumerate().for_each(|(i, input)| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(input);
            verifier_state.checked_public_inputs[2 + i] = arr
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
            LightInstructionThird<'info, { VERIFYINGKEY_ROCK_PAPER_SCISSORS.nr_pubinputs }>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let verifier_state = ctx.accounts.verifier_state.load()?;
        let mut reversed_public_inputs = verifier_state.checked_public_inputs[2];
        reversed_public_inputs.reverse();
        if reversed_public_inputs
            != ctx
                .accounts
                .game_pda
                .game
                .player_one_program_utxo
                .gameCommitmentHash
                .x
        {
            for (idx, val) in verifier_state.checked_public_inputs.iter().enumerate() {
                msg!("Public input {}={:?}", idx, val);
            }

            msg!("{:?}", verifier_state.checked_public_inputs);
            msg!(
                "{:?}",
                ctx.accounts
                    .game_pda
                    .game
                    .player_one_program_utxo
                    .gameCommitmentHash
            );
            panic!("player_one_program_utxo does not match");
        }
        let mut reversed_public_inputs = verifier_state.checked_public_inputs[3];
        reversed_public_inputs.reverse();
        if reversed_public_inputs
            != ctx
                .accounts
                .game_pda
                .game
                .player_two_program_utxo
                .unwrap()
                .gameCommitmentHash
                .x
        {
            msg!("{:?}", verifier_state.checked_public_inputs);
            msg!(
                "{:?}",
                ctx.accounts
                    .game_pda
                    .game
                    .player_two_program_utxo
                    .unwrap()
                    .gameCommitmentHash
            );
            panic!("player_two_program_utxo does not match");
        }
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
            CloseVerifierState<'info, { VERIFYINGKEY_ROCK_PAPER_SCISSORS.nr_pubinputs }>,
        >,
    ) -> Result<()> {
        Ok(())
    }

    pub fn create_game<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, CreateGameInstruction<'info>>,
        utxo_bytes: Vec<u8>,
    ) -> Result<()> {
        let utxo = UtxoInternal::deserialize(&mut utxo_bytes.as_slice())?;
        msg!(
            "gameCommitmentHash {:?}",
            utxo.gameCommitmentHash.x.as_slice()
        );
        let gch: [u8; 32] = utxo
            .gameCommitmentHash
            .x
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        msg!("gch as [u8;32] = {:?}", gch);
        msg!("find_program_address {:?}", utxo);

        ctx.accounts.game_pda.game = Game::new(utxo.try_into().unwrap());
        Ok(())
    }

    pub fn join_game<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, JoinGameInstruction<'info>>,
        utxo_bytes: Vec<u8>,
        choice: u8,
        slot: u64,
    ) -> Result<()> {
        let utxo: UtxoInternal = UtxoInternal::deserialize(&mut utxo_bytes.as_slice())?;
        ctx.accounts.game_pda.game.join(utxo, choice, slot);
        Ok(())
    }

    pub fn close_game(_ctx: Context<CloseGame>) -> Result<()> {
        Ok(())
    }
}
