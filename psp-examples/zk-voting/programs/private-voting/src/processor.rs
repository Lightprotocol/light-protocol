use crate::verifying_key_private_voting::VERIFYINGKEY_PRIVATE_VOTING;
use crate::LightInstructionThird;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use light_macros::pubkey;
use light_verifier_sdk::light_transaction::Proof;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::{light_app_transaction::AppTransaction, light_transaction::Config};
#[derive(Clone)]
pub struct TransactionsConfig;
impl Config for TransactionsConfig {
    /// ProgramId.
    const ID: Pubkey = pubkey!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
}

#[inline(never)]
pub fn cpi_verifier_two<'a, 'b, 'c, 'info, const NR_CHECKED_INPUTS: usize>(
    ctx: &'a Context<'a, 'b, 'c, 'info, LightInstructionThird<'info, NR_CHECKED_INPUTS>>,
    inputs: &'a Vec<u8>,
) -> Result<()> {
    let proof_verifier = Proof {
        a: inputs[256..256 + 64].try_into().unwrap(),
        b: inputs[256 + 64..256 + 192].try_into().unwrap(),
        c: inputs[256 + 192..256 + 256].try_into().unwrap(),
    };

    let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[
            ctx.accounts.signing_address.key().to_bytes().as_ref(),
            VERIFIER_STATE_SEED.as_ref(),
        ],
        ctx.program_id,
    );

    let bump = &[bump];
    let seed = &ctx.accounts.signing_address.key().to_bytes();
    let domain_separation_seed = VERIFIER_STATE_SEED;
    let cpi_seed = &[seed, domain_separation_seed, &bump[..]];
    let final_seed = &[&cpi_seed[..]];

    let accounts: light_psp4in4out_app_storage::cpi::accounts::LightInstruction<'info> =
        light_psp4in4out_app_storage::cpi::accounts::LightInstruction {
            verifier_state: ctx.accounts.verifier_state.to_account_info(),
            signing_address: ctx.accounts.signing_address.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            registered_verifier_pda: ctx.accounts.registered_verifier_pda.to_account_info(),
            program_merkle_tree: ctx.accounts.program_merkle_tree.to_account_info(),
            transaction_merkle_tree: ctx.accounts.transaction_merkle_tree.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            sender_spl: ctx.accounts.sender_spl.to_account_info(),
            recipient_spl: ctx.accounts.recipient_spl.to_account_info(),
            sender_sol: ctx.accounts.sender_sol.to_account_info(),
            recipient_sol: ctx.accounts.recipient_sol.to_account_info(),
            // relayer recipient and escrow will never be used in the same transaction
            relayer_recipient_sol: ctx.accounts.relayer_recipient_sol.to_account_info(),
            token_authority: ctx.accounts.token_authority.to_account_info(),
            log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
            event_merkle_tree: ctx.accounts.event_merkle_tree.to_account_info(),
        };

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.verifier_program.to_account_info(),
        accounts,
        &final_seed[..],
    );
    cpi_ctx = cpi_ctx.with_remaining_accounts(ctx.remaining_accounts.to_vec());
    let verifier_state = ctx.accounts.verifier_state.load()?;
    light_psp4in4out_app_storage::cpi::shielded_transfer_inputs(
        cpi_ctx,
        proof_verifier.a,
        proof_verifier.b,
        proof_verifier.c,
        <Vec<u8> as TryInto<[u8; 32]>>::try_into(verifier_state.checked_public_inputs[1].to_vec())
            .unwrap(),
        memoffset::offset_of!(crate::psp_accounts::VerifierState, verifier_state_data),
    )
}
#[inline(never)]
pub fn verify_programm_proof<'a, 'b, 'c, 'info, const NR_CHECKED_INPUTS: usize>(
    ctx: &'a Context<'a, 'b, 'c, 'info, LightInstructionThird<'info, NR_CHECKED_INPUTS>>,
    inputs: &'a Vec<u8>,
) -> Result<()> {
    let proof_app = Proof {
        a: inputs[0..64].try_into().unwrap(),
        b: inputs[64..192].try_into().unwrap(),
        c: inputs[192..256].try_into().unwrap(),
    };
    let verifier_state = ctx.accounts.verifier_state.load()?;
    const NR_CHECKED_INPUTS: usize = VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs;
    let mut app_verifier = AppTransaction::<NR_CHECKED_INPUTS, TransactionsConfig>::new(
        &proof_app,
        &verifier_state.checked_public_inputs,
        &VERIFYINGKEY_PRIVATE_VOTING,
    );

    app_verifier.verify()
}

#[inline(never)]
pub fn check_public_inputs<'a, 'b, 'c, 'info>(
    ctx: &'a Context<
        'a,
        'b,
        'c,
        'info,
        LightInstructionThird<'info, { VERIFYINGKEY_PRIVATE_VOTING.nr_pubinputs }>,
    >,
) -> Result<()> {
    let mut vote_pda = ctx.accounts.vote_pda.load_mut()?;
    let verifier_state = ctx.accounts.verifier_state.load()?;
    // TODO: implement automatic from_array method for public inputs struct, could use types here standarad 254, also available u64

    let public_vote_weight_yes_x = verifier_state.checked_public_inputs[2];
    let public_vote_weight_yes_y = verifier_state.checked_public_inputs[3];
    let public_vote_weight_yes_emphemeral_x = verifier_state.checked_public_inputs[4];
    let public_vote_weight_yes_emphemeral_y = verifier_state.checked_public_inputs[5];
    let public_vote_weight_no_x = verifier_state.checked_public_inputs[6];
    let public_vote_weight_no_y = verifier_state.checked_public_inputs[7];
    let public_vote_weight_no_emphemeral_x = verifier_state.checked_public_inputs[8];
    let public_vote_weight_no_emphemeral_y = verifier_state.checked_public_inputs[9];
    let public_elgamal_public_key_x = verifier_state.checked_public_inputs[10];
    let public_elgamal_public_key_y = verifier_state.checked_public_inputs[11];
    let public_vote_end = verifier_state.checked_public_inputs[12];
    let public_mint = verifier_state.checked_public_inputs[13];
    let public_vote_weight_psp_address = verifier_state.checked_public_inputs[14];
    // no need to check vote weight nullifier if nullifie exists nullifier pda create fails
    // let vote_weight_nullifier = verifier_state.checked_public_inputs[15];
    // let public_vote_id = verifier_state.checked_public_inputs[16];
    let public_current_slot = verifier_state.checked_public_inputs[17];
    let public_old_vote_weight_yes_x = verifier_state.checked_public_inputs[18];
    let public_old_vote_weight_yes_y = verifier_state.checked_public_inputs[19];
    let public_old_vote_weight_yes_emphemeral_x = verifier_state.checked_public_inputs[20];
    let public_old_vote_weight_yes_emphemeral_y = verifier_state.checked_public_inputs[21];
    let public_old_vote_weight_no_x = verifier_state.checked_public_inputs[22];
    let public_old_vote_weight_no_y = verifier_state.checked_public_inputs[23];
    let public_old_vote_weight_no_emphemeral_x = verifier_state.checked_public_inputs[24];
    let public_old_vote_weight_no_emphemeral_y = verifier_state.checked_public_inputs[25];

    let current_slot = Clock::get()?.slot;
    if current_slot - 50 > be_u64_from_public_input(&public_current_slot) {
        msg!(
            "Current slot does not match, expected {} to be greater than {}",
            current_slot - 50,
            be_u64_from_public_input(&public_current_slot)
        );
        panic!("Current slot does not match");
        // return Err(ErrorCode::CurrentSlotMismatch.into());
    }

    // elGamal public key needs to equal vote pda
    let hashed_governing_token_mint = [
        vec![0u8],
        hash(&vote_pda.governing_token_mint.to_bytes()).try_to_vec()?[1..].to_vec(),
    ]
    .concat();
    if public_mint != [0u8; 32] && hashed_governing_token_mint != public_mint.to_vec() {
        msg!(
            "Governing token mint does not match, expected {:?}, got {:?}",
            vote_pda.governing_token_mint,
            public_mint
        );
        panic!("Governing token mint does not match");
        // return Err(ErrorCode::GoverningTokenMintMismatch.into());
    }
    let threshold_encryption_pubkey: [u8; 64] =
        [public_elgamal_public_key_x, public_elgamal_public_key_y]
            .concat()
            .try_into()
            .unwrap();
    if vote_pda.threshold_encryption_pubkey != threshold_encryption_pubkey {
        msg!(
            "ElGamal public key does not match, expected {:?}, got {:?}",
            vote_pda.threshold_encryption_pubkey,
            threshold_encryption_pubkey
        );
        panic!("ElGamal public key does not match");
        // return Err(ErrorCode::ElGamalPublicKeyMismatch.into());
    }
    if vote_pda.slot_vote_end != be_u64_from_public_input(&public_vote_end) {
        msg!(
            "Voting end does not match, expected {}, got {}",
            vote_pda.slot_vote_end,
            be_u64_from_public_input(&public_vote_end)
        );
        panic!("Voting end does not match");
        // return Err(ErrorCode::VotingEndMismatch.into());
    }

    let hashed_vote_weight_psp_address = [
        vec![0u8],
        hash(&vote_pda.vote_weight_psp.to_bytes()).try_to_vec()?[1..].to_vec(),
    ]
    .concat();
    if hashed_vote_weight_psp_address != public_vote_weight_psp_address.to_vec() {
        msg!("Vote weight psp address: {:?}", vote_pda.vote_weight_psp);
        msg!(
            "Hashed vote weight psp address does not match, expected {:?}, got {:?}",
            hashed_vote_weight_psp_address,
            public_vote_weight_psp_address
        );
        panic!("Vote weight psp address does not match");
        // return Err(ErrorCode::VoteWeightPspAddressMismatch.into());
    }
    // public_old_vote_weight_yes_x
    // public_old_vote_weight_yes_y
    // public_old_vote_weight_yes_emphemeral_x
    // public_old_vote_weight_yes_emphemeral_y
    // public_old_vote_weight_no_x
    // public_old_vote_weight_no_y
    // public_old_vote_weight_no_emphemeral_x
    // public_old_vote_weight_no_emphemeral_y
    if vote_pda.encrypted_yes_votes
        != [
            public_old_vote_weight_yes_emphemeral_x,
            public_old_vote_weight_yes_emphemeral_y,
            public_old_vote_weight_yes_x,
            public_old_vote_weight_yes_y,
        ]
    {
        msg!(
            "Encrypted old yes votes do not match, expected {:?}, got {:?}",
            vote_pda.encrypted_yes_votes,
            [
                public_old_vote_weight_yes_emphemeral_x,
                public_old_vote_weight_yes_emphemeral_y,
                public_old_vote_weight_yes_x,
                public_old_vote_weight_yes_y,
            ]
        );
        panic!("Encrypted old yes votes do not match");
    }

    if vote_pda.encrypted_no_votes
        != [
            public_old_vote_weight_no_emphemeral_x,
            public_old_vote_weight_no_emphemeral_y,
            public_old_vote_weight_no_x,
            public_old_vote_weight_no_y,
        ]
    {
        msg!(
            "Encrypted old no votes do not match, expected {:?}, got {:?}",
            vote_pda.encrypted_no_votes,
            [
                public_old_vote_weight_no_emphemeral_x,
                public_old_vote_weight_no_emphemeral_y,
                public_old_vote_weight_no_x,
                public_old_vote_weight_no_y,
            ]
        );
        panic!("Encrypted old no votes do not match");
    }

    vote_pda.encrypted_yes_votes = [
        public_vote_weight_yes_emphemeral_x,
        public_vote_weight_yes_emphemeral_y,
        public_vote_weight_yes_x,
        public_vote_weight_yes_y,
    ];
    vote_pda.encrypted_no_votes = [
        public_vote_weight_no_emphemeral_x,
        public_vote_weight_no_emphemeral_y,
        public_vote_weight_no_x,
        public_vote_weight_no_y,
    ];
    Ok(())
}

pub fn be_u64_from_public_input(input: &[u8; 32]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&input[24..32]);
    u64::from_be_bytes(arr)
}
