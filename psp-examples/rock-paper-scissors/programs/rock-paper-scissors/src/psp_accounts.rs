use crate::u256;
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
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 3000, payer = signing_address )]
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
    pub rpc_fee: u64,
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
    #[account(mut, seeds = [LightMerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program=LightPsp4in4outAppStorage::id())]
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
    /// CHECK:` Is not checked the rpc has complete freedom.
    #[account(mut)]
    pub rpc_recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the rpc has complete freedom.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the rpc has complete freedom.
    #[account(mut, seeds= [LightPsp4in4outAppStorage::id().to_bytes().as_ref()], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, LightPsp4in4outAppStorage>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    #[account(mut)]
    pub game_pda: Box<Account<'info, GamePda>>,
}

// const GAME_PDA_SEED: &[u8] = b"game_pda";
// &utxo.gameCommitmentHash.x.as_slice(),

#[allow(non_snake_case)]
#[derive(Accounts)]
#[instruction(game_commitment_hash: Vec<u8>)]
pub struct CreateGameInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[allow(non_snake_case)]
    #[account(
        init,
        seeds = [&game_commitment_hash[game_commitment_hash.len()-64..game_commitment_hash.len()-32]],
        // seeds = [GAME_PDA_SEED],
        bump,
        payer = signer,
        space = 3000)
    ]
    pub game_pda: Account<'info, GamePda>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
// #[instruction(game_commitment_hash: Vec<u8>)]
pub struct CloseGame<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        // seeds = [GAME_PDA_SEED], 
        // seeds = [&game_commitment_hash[game_commitment_hash.len()-32..]],
        // bump,
        close=signer
    )]
    pub game_pda: Account<'info, GamePda>,
}

#[derive(Accounts)]
pub struct JoinGameInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    // #[allow(non_snake_case)]
    #[account(mut)]
    pub game_pda: Account<'info, GamePda>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug)]
#[account]
pub struct GamePda {
    pub game: Game,
}

#[allow(non_snake_case)]
#[derive(Debug, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Clone)]
pub struct UtxoInternal {
    pub amounts: [u64; 2],
    pub spl_asset_index: u64,
    pub verifier_address_index: u64,
    pub blinding: u256,
    pub app_data_hash: u256,
    pub account_shielded_public_key: u256,
    pub account_encryption_public_key: [u8; 32],
    pub gameCommitmentHash: u256,
    pub userPubkey: u256,
}

#[derive(Debug, AnchorDeserialize, AnchorSerialize, Clone)]
pub struct Game {
    pub player_one_program_utxo: UtxoInternal,
    pub player_two_program_utxo: Option<UtxoInternal>,
    pub player_two_choice: Option<u8>,
    _padding: [u8; 7],
    pub slot: Option<u64>,
    pub is_joinable: bool,
    pub _padding2: [u8; 7],
}

impl Game {
    pub fn new(player_one_program_utxo: UtxoInternal) -> Self {
        Self {
            player_one_program_utxo,
            player_two_program_utxo: None,
            slot: None,
            player_two_choice: None,
            _padding: [0u8; 7],
            is_joinable: true,
            _padding2: [0u8; 7],
        }
    }

    pub fn join(&mut self, player_two_program_utxo: UtxoInternal, choice: u8, slot: u64) {
        self.player_two_program_utxo = Some(player_two_program_utxo);
        self.is_joinable = false;
        self.player_two_choice = Some(choice);
        self.slot = Some(slot);
    }
}

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

#[account(zero_copy)]
pub struct VerifierState {
    pub signer: Pubkey,
    pub verifier_state_data: [u8; 1024],
    pub checked_public_inputs: [[u8; 32]; 4],
}
