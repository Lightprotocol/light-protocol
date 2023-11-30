use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;

pub mod psp_accounts;
pub use psp_accounts::*;
pub mod auto_generated_accounts;
pub use auto_generated_accounts::*;
pub mod processor;
pub use processor::*;
pub mod verifying_key_init_vote;
pub mod verifying_key_private_voting;
pub mod verifying_key_publish_decrypted_tally;
use light_psp4in4out_app_storage::Psp4In4OutAppStorageVerifierState;
use light_verifier_sdk::light_app_transaction::AppTransaction;
use light_verifier_sdk::light_transaction::Proof;
use serde::{Deserialize, Serialize};
use solana_program::program::invoke;
use verifying_key_init_vote::VERIFYINGKEY_INIT_VOTE;
pub use verifying_key_private_voting::*;
use verifying_key_publish_decrypted_tally::VERIFYINGKEY_PUBLISH_DECRYPTED_TALLY;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[constant]
pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

#[program]
pub mod private_voting {

    use super::*;
    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during{ VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn light_instruction_first<'a, 'b, 'c, 'info>(
        ctx: Context<
            'a,
            'b,
            'c,
            'info,
            LightInstructionFirst<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
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
            LightInstructionSecond<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
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
            LightInstructionThird<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
        >,
        inputs: Vec<u8>,
    ) -> Result<()> {
        check_vote(&ctx)?;
        check_public_inputs(&ctx)?;
        verify_programm_proof(&ctx, &inputs)?;
        cpi_verifier_two(&ctx, &inputs)
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<
            'a,
            'b,
            'c,
            'info,
            CloseVerifierState<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
        >,
    ) -> Result<()> {
        Ok(())
    }

    /// Creates a mock proposal
    pub fn init_mock_proposal<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitMockProposal>,
        governing_token_mint: Pubkey,
        start_voting_at: u64,
        voting_completed_at: u64,
        max_vote_weight: u64,
        vote_threshold: u64,
        name: String,
        veto_vote_weight: u64,
    ) -> Result<()> {
        ctx.accounts.proposal.governing_token_mint = governing_token_mint;
        ctx.accounts.proposal.start_voting_at = Some(start_voting_at);
        ctx.accounts.proposal.voting_completed_at = Some(voting_completed_at);
        ctx.accounts.proposal.max_vote_weight = Some(max_vote_weight);
        ctx.accounts.proposal.vote_threshold = Some(vote_threshold);
        ctx.accounts.proposal.name = name;
        ctx.accounts.proposal.veto_vote_weight = veto_vote_weight;

        Ok(())
    }

    // TODO: add a proof that cipher text encrypts zero
    /// Initializes a vote based on a proposal
    pub fn init_vote<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitVote>,
        elgamal_public_key_x: [u8; 32],
        elgamal_public_key_y: [u8; 32],
        encrypted_zero_point_ephemeral_key_x: [u8; 32],
        encrypted_zero_point_ephemeral_key_y: [u8; 32],
        encrypted_zero_point_x: [u8; 32],
        encrypted_zero_point_y: [u8; 32],
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) -> Result<()> {
        let mut vote_pda = ctx.accounts.vote_pda.load_init()?;
        vote_pda.proposal_pda = ctx.accounts.proposal.key();
        vote_pda.governing_token_mint = ctx.accounts.proposal.governing_token_mint;
        vote_pda.slot_vote_start = ctx.accounts.proposal.start_voting_at.unwrap();
        vote_pda.slot_vote_end = ctx.accounts.proposal.voting_completed_at.unwrap();
        vote_pda.max_vote_weight = ctx.accounts.proposal.max_vote_weight.unwrap();
        vote_pda.vote_weight_psp = *ctx.program_id;
        vote_pda.threshold_encryption_pubkey = [elgamal_public_key_x, elgamal_public_key_y]
            .concat()
            .try_into()
            .unwrap();
        vote_pda.encrypted_yes_votes = [
            encrypted_zero_point_ephemeral_key_x,
            encrypted_zero_point_ephemeral_key_y,
            encrypted_zero_point_x,
            encrypted_zero_point_y,
        ];
        vote_pda.encrypted_no_votes = [
            encrypted_zero_point_ephemeral_key_x,
            encrypted_zero_point_ephemeral_key_y,
            encrypted_zero_point_x,
            encrypted_zero_point_y,
        ];
        let public_inputs = [
            elgamal_public_key_x,
            elgamal_public_key_y,
            encrypted_zero_point_ephemeral_key_x,
            encrypted_zero_point_ephemeral_key_y,
            encrypted_zero_point_x,
            encrypted_zero_point_y,
        ];
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
        let mut verifier = AppTransaction::<
            { VERIFYINGKEY_INIT_VOTE.nr_pubinputs },
            processor::TransactionsConfig,
        >::new(&proof, &public_inputs, &VERIFYINGKEY_INIT_VOTE);
        verifier.verify()
    }

    /// Publishes the decrypted tally and proves correct decryption and decoding
    pub fn publish_decrypted_tally<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, PublishDecryptedTally>,
        publicYesResult: [u8; 32],
        publicNoResult: [u8; 32],
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) -> Result<()> {
        let mut vote_pda = ctx.accounts.vote_pda.load_mut()?;
        let publicVoteWeightYesX = vote_pda.encrypted_yes_votes[2];
        let publicVoteWeightYesY = vote_pda.encrypted_yes_votes[3];
        let publicVoteWeightYesEmphemeralKeyX = vote_pda.encrypted_yes_votes[0];
        let publicVoteWeightYesEmphemeralKeyY = vote_pda.encrypted_yes_votes[1];
        let publicVoteWeightNoX = vote_pda.encrypted_no_votes[2];
        let publicVoteWeightNoY = vote_pda.encrypted_no_votes[3];
        let publicVoteWeightNoEmphemeralKeyX = vote_pda.encrypted_no_votes[0];
        let publicVoteWeightNoEmphemeralKeyY = vote_pda.encrypted_no_votes[1];
        let public_inputs = [
            publicVoteWeightYesX,
            publicVoteWeightYesY,
            publicVoteWeightYesEmphemeralKeyX,
            publicVoteWeightYesEmphemeralKeyY,
            publicVoteWeightNoX,
            publicVoteWeightNoY,
            publicVoteWeightNoEmphemeralKeyX,
            publicVoteWeightNoEmphemeralKeyY,
            publicYesResult,
            publicNoResult,
        ];
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
        let mut verifier = AppTransaction::<
            { VERIFYINGKEY_PUBLISH_DECRYPTED_TALLY.nr_pubinputs },
            processor::TransactionsConfig,
        >::new(
            &proof,
            &public_inputs,
            &VERIFYINGKEY_PUBLISH_DECRYPTED_TALLY,
        );
        verifier.verify()?;
        vote_pda.decrypted_yes_vote_weight = be_u64_from_public_input(&publicYesResult).clone();
        vote_pda.decrypted_no_vote_weight = be_u64_from_public_input(&publicNoResult).clone();
        Ok(())
    }
    // /// cpi which passes a transaction through so that the vote psp has utxo ownership
    // pub fn modify_vote_weight_utxo_cpi<'a, 'b, 'c, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, InitVote>,
    // ) -> Result<()> {
    //     Ok(())
    // }

    // -------------- Move to voteWeight PSP ----------------
    // /// Initializes a vote based on a proposal
    // pub fn modify_vote_weight_utxo_cpi<'a, 'b, 'c, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, InitVote>,
    // ) -> Result<()> {
    //     Ok(())
    // }
    // /// Initializes a vote based on a proposal
    // pub fn init_vote<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, InitVote>) -> Result<()> {
    //     ctx.accounts.vote_pda.proposal_pda = ctx.accounts.proposal.key();
    //     ctx.accounts.vote_pda.governing_token_mint = ctx.accounts.proposal.governing_token_mint;
    //     ctx.accounts.vote_pda.slot_vote_start = ctx.accounts.proposal.start_voting_at.unwrap();
    //     ctx.accounts.vote_pda.slot_vote_end = ctx.accounts.proposal.voting_completed_at.unwrap();
    //     ctx.accounts.vote_pda.max_vote_weight = ctx.accounts.proposal.max_vote_weight.unwrap();
    //     Ok(())
    // }

    // /// Initializes a vote based on a proposal
    // pub fn init_vote_weight_config(
    //     ctx: Context<'a, 'b, 'c, 'info, InitVoteWeightConfig>,
    // ) -> Result<()> {
    //     Ok(())
    // }

    // TODO: check whether we need this instruction at all
    pub fn create_vote_weight_instruction_first<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, VerifyCreateVoteWeightInstructionFirst<'info>>,
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

    // check whether we need this instruction at all
    pub fn create_vote_weight_instruction_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, VerifyCreateVoteWeightInstructionSecond<'info>>,
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
    pub fn create_vote_weight_instruction_third<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, VerifyCreateVoteWeightInstructionThird<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        // should only be called via cpi from the vote program and only verify the program proof
        // the system proof is verified via cpi directly from the vote program
        verify_create_vote_weight_cpi(&ctx, &inputs)?;

        cpi_verifier_two_create_vote_utxo(&ctx, &inputs)
    }

    // /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    // pub fn close_verifier_state<'a, 'b, 'c, 'info>(
    //     _ctx: Context<
    //         'a,
    //         'b,
    //         'c,
    //         'info,
    //         CloseVerifierState<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
    //     >,
    // ) -> Result<()> {
    //     Ok(())
    // }
}

fn verify_create_vote_weight_cpi<'a, 'b, 'c, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, VerifyCreateVoteWeightInstructionThird<'info>>,
    inputs: &'a Vec<u8>,
) -> Result<()> {
    let verifier_state_id = ctx.accounts.verifier_state.key().clone();
    let verifier_state_account_info = ctx.accounts.verifier_state.to_account_info().clone();

    let instruction_data = {
        let mut checked_public_inputs = ctx
            .accounts
            .verifier_state
            .load()?
            .checked_public_inputs
            .clone();
        let mut program_id_hash = hash(&ctx.accounts.vote_weight_program.key.to_bytes()).to_bytes();
        program_id_hash[0] = 0;
        checked_public_inputs[2] = program_id_hash;
        msg!("checked_public_inputs {:?}", checked_public_inputs);
        // instruction discriminator
        let mut instruction_data = vec![101, 200, 46, 73, 4, 61, 156, 75];
        // proof
        instruction_data.extend_from_slice(&inputs[0..256]);
        // public inputs
        instruction_data.extend(checked_public_inputs.into_iter().flat_map(|x| x));
        instruction_data
    };

    let account_meta: AccountMeta = AccountMeta {
        pubkey: *ctx.accounts.vote_weight_config.key,
        is_signer: false,
        is_writable: false,
    };
    let instruction = solana_program::instruction::Instruction {
        program_id: *ctx.accounts.vote_weight_program.key,
        data: instruction_data,
        accounts: vec![account_meta],
    };

    invoke(
        &instruction,                               // vote program
        &[ctx.accounts.vote_weight_config.clone()], // acounts to pass to the cpi
    )?;
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct VerifyCreateVoteWeightCpi {
    pub discriminator: [u8; 8],
    pub proof: Vec<u8>,
    pub public_inputs: [[u8; 32]; 20],
}

#[inline(never)]
fn check_vote<'a, 'b, 'c, 'info>(
    ctx: &'a Context<
        'a,
        'b,
        'c,
        'info,
        LightInstructionThird<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
    >,
) -> Result<()> {
    let vote_pda = ctx.accounts.vote_pda.load()?;
    let current_slot = Clock::get()?.slot;
    if current_slot < vote_pda.slot_vote_start {
        msg!(
            "Voting not started yet, current slot: {} starting slot {}",
            current_slot,
            vote_pda.slot_vote_start
        );
        // return Err(ErrorCode::VotingNotStarted.into());
        panic!("Voting not started yet");
    }
    if current_slot > vote_pda.slot_vote_end {
        msg!(
            "Voting completed, current slot: {} ending slot {}",
            current_slot,
            vote_pda.slot_vote_end
        );
        panic!("Voting completed");
        // return Err(ErrorCode::VotingCompleted.into());
    }
    Ok(())
}
