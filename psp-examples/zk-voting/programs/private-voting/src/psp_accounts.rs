use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_merkle_tree_program::transaction_merkle_tree::state::TransactionMerkleTree;
use light_merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use light_merkle_tree_program::{program::LightMerkleTreeProgram, EventMerkleTree};
use light_psp4in4out_app_storage::{self, program::LightPsp4in4outAppStorage};
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 4000, payer = signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionFirst {
    pub public_amount_spl: [u8; 32],
    pub input_nullifier: [[u8; 32]; 4],
    pub output_commitment: [[u8; 32]; 4],
    pub public_amount_sol: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub root_index: u64,
    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
}

#[derive(Accounts)]
pub struct LightInstructionSecond<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionThird<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.load().unwrap().signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, LightMerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [LightMerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program= LightPsp4in4outAppStorage::id())]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_sol: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= LightMerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [LightPsp4in4outAppStorage::id().to_bytes().as_ref()], bump, seeds::program= LightMerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, LightPsp4in4outAppStorage>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    #[account(mut)]
    pub vote_pda: AccountLoader<'info, VotePda>,
    #[account(init, seeds = [&verifier_state.load().unwrap().checked_public_inputs[15].as_slice()], bump, space=8, payer = signing_address )]
    pub nullifier_pda: Account<'info, NullifierPda>,
}
#[account]
pub struct NullifierPda {}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionThird {
    pub proof_a_app: [u8; 64],
    pub proof_b_app: [u8; 128],
    pub proof_c_app: [u8; 64],
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.load().unwrap().signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

#[derive(Accounts)]
pub struct InitMockProposal<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(init, seeds = [&signer.key().to_bytes(), b"MockProposalV2".as_slice()], bump, space=2000, payer = signer )]
    pub proposal: Account<'info, MockProposalV2>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug)]
#[account]
pub struct MockProposalV2 {
    // /// Governance account type
    // pub account_type: GovernanceAccountType,
    // /// Governance account the Proposal belongs to
    // pub governance: Pubkey,
    /// Indicates which Governing Token is used to vote on the Proposal
    /// Whether the general Community token owners or the Council tokens owners vote on this Proposal
    pub governing_token_mint: Pubkey,

    // /// Current proposal state
    // pub state: ProposalState,

    // // TODO: add state_at timestamp to have single field to filter recent proposals in the UI
    // /// The TokenOwnerRecord representing the user who created and owns this Proposal
    // pub token_owner_record: Pubkey,

    // /// The number of signatories assigned to the Proposal
    // pub signatories_count: u8,

    // /// The number of signatories who already signed
    // pub signatories_signed_off_count: u8,
    // /// Vote type
    // pub vote_type: VoteType,
    // /// Proposal options
    // pub options: Vec<ProposalOption>,
    /// The total weight of the Proposal rejection votes
    /// If the proposal has no deny option then the weight is None
    ///
    /// Only proposals with the deny option can have executable instructions attached to them
    /// Without the deny option a proposal is only non executable survey
    ///
    /// The deny options is also used for off-chain and/or manually executable proposal to make them binding
    /// as opposed to survey only proposals
    pub deny_vote_weight: Option<u64>,

    /// Reserved space for future versions
    /// This field is a leftover from unused veto_vote_weight: Option<u64>
    pub reserved1: u8,

    /// The total weight of  votes
    /// Note: Abstain is not supported in the current version
    pub abstain_vote_weight: Option<u64>,

    /// Optional start time if the Proposal should not enter voting state immediately after being signed off
    /// Note: start_at is not supported in the current version
    pub start_voting_at: Option<u64>,

    // // /// When the Proposal was created and entered Draft state
    // // pub draft_at: u64,
    // /// When Signatories started signing off the Proposal
    // // pub signing_off_at: Option<u64>,
    // /// When the Proposal began voting as u64
    // pub voting_at: Option<u64>,
    // /// When the Proposal began voting as Slot
    // /// Note: The slot is not currently used but the exact slot is going to be required to support snapshot based vote weights
    // pub voting_at_slot: Option<Slot>,
    /// When the Proposal ended voting and entered either Succeeded or Defeated
    pub voting_completed_at: Option<u64>,

    // /// When the Proposal entered Executing state
    // pub executing_at: Option<u64>,
    // /// When the Proposal entered final state Completed or Cancelled and was closed
    // pub closed_at: Option<u64>,

    // /// Instruction execution flag for ordered and transactional instructions
    // /// Note: This field is not used in the current version
    // pub execution_flags: InstructionExecutionFlags,
    /// The max vote weight for the Governing Token mint at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the mint supply or max weight source changed
    /// after vote was completed.
    pub max_vote_weight: Option<u64>,

    /// Max voting time for the proposal if different from parent Governance  (only higher value possible)
    /// Note: This field is not used in the current version
    pub max_voting_time: Option<u32>,

    /// The vote threshold at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the threshold
    /// was changed for governance config after vote was completed.
    /// TODO: Use this field to override the threshold from parent Governance (only higher value possible)
    pub vote_threshold: Option<u64>,

    /// Reserved space for future versions
    pub reserved: [u8; 64],

    /// Proposal name
    pub name: String,

    // /// Link to proposal's description
    // pub description_link: String,
    /// The total weight of Veto votes
    pub veto_vote_weight: u64,
}

#[derive(Accounts)]
pub struct InitVote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub proposal: Account<'info, MockProposalV2>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&proposal.key().to_bytes(), b"VOTE".as_slice()], bump, space=2000, payer = signer )]
    pub vote_pda: AccountLoader<'info, VotePda>,
}

#[derive(Accounts)]
pub struct PublishDecryptedTally<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub proposal: Account<'info, MockProposalV2>,
    pub system_program: Program<'info, System>,
    #[account(mut, seeds = [&proposal.key().to_bytes(), b"VOTE".as_slice()], bump )]
    pub vote_pda: AccountLoader<'info, VotePda>,
}

#[derive(Debug)]
#[account(zero_copy)]
pub struct VotePda {
    pub proposal_pda: Pubkey,
    pub governing_token_mint: Pubkey,
    pub threshold_encryption_pubkey: [u8; 64],
    pub slot_vote_start: u64,
    pub slot_vote_end: u64,
    pub max_vote_weight: u64,
    pub encrypted_yes_votes: [[u8; 32]; 4], // first 2 is the empheemeral key (curve point), second 2 is the ciphertext (curve point)
    pub encrypted_no_votes: [[u8; 32]; 4],
    pub decrypted_yes_vote_weight: u64,
    pub decrypted_no_vote_weight: u64,
    pub vote_weight_psp: Pubkey,
}

#[account(zero_copy)]
pub struct VerifierState {
    pub signer: Pubkey,
    pub verifier_state_data: [u8; 1024], //VerifierState,
    pub checked_public_inputs: [[u8; 32]; 26],
}
