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

pub mod processor;
pub mod verifying_key;
pub use processor::*;

use crate::processor::TransactionsConfig;
use crate::processor::{
    process_transfer_4_ins_4_outs_4_checked_first, process_transfer_4_ins_4_outs_4_checked_second,
};
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::{light_transaction::VERIFIER_STATE_SEED, state::VerifierState10Ins};
use merkle_tree_program::program::MerkleTreeProgram;
use merkle_tree_program::transaction_merkle_tree::state::TransactionMerkleTree;
use merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use verifier_program_two::{self, program::VerifierProgramTwo};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum MarketPlaceError {
    #[msg("The offer expired.")]
    OfferExpired,
}
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
        ];
        let leaves = [
            [inputs_des.leaves[0], inputs_des.leaves[1]],
            [inputs_des.leaves[2], inputs_des.leaves[3]],
        ];
        process_transfer_4_ins_4_outs_4_checked_first(
            ctx,
            &proof_a,
            &proof_b,
            &proof_c,
            &inputs_des.public_amount_spl,
            &inputs_des.nullifiers,
            &leaves,
            &inputs_des.public_amount_sol,
            &checked_inputs,
            &inputs_des.encrypted_utxos,
            &pool_type,
            &inputs_des.root_index,
            &inputs_des.relayer_fee,
        )
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn light_instruction_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        msg!("original {:?}", inputs[0..32].to_vec());
        let inputs_des: InstructionDataLightInstructionSecond =
            InstructionDataLightInstructionSecond::try_deserialize(&mut inputs.as_slice())?;

        // ctx.accounts.verifier_state.checked_public_inputs.insert(
        //     0,
        //     [
        //         vec![0u8],
        //         hash(&ctx.program_id.to_bytes()).try_to_vec()?[1..].to_vec(),
        //     ]
        //     .concat(),
        // );
        // ctx.accounts
        //     .verifier_state
        //     .checked_public_inputs
        //     .insert(1, inputs.transaction_hash);
        msg!("inputs {:?}", inputs_des);
        process_transfer_4_ins_4_outs_4_checked_second(
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

// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 3000/*8 + 32 * 6 + 10 * 32 + 2 * 32 + 512 + 16 + 128*/, payer = signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionsConfig>>>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionFirst {
    public_amount_spl: [u8; 32],
    nullifiers: [[u8; 32]; 4],
    leaves: [[u8; 32]; 4],
    public_amount_sol: [u8; 32],
    transaction_hash: [u8; 32],
    root_index: u64,
    relayer_fee: u64,
    encrypted_utxos: Vec<u8>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionSecond<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionsConfig>>>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program= VerifierProgramTwo::id())]
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
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [VerifierProgramTwo::id().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, VerifierProgramTwo>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionSecond {
    proof_a_app: [u8; 64],
    proof_b_app: [u8; 128],
    proof_c_app: [u8; 64],
    proof_a: [u8; 64],
    proof_b: [u8; 128],
    proof_c: [u8; 64],
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionsConfig>>>,
}

#[allow(non_camel_case_types)]
// helper struct to create anchor idl with u256 type
#[account]
pub struct u256 {
    x: [u8; 32],
}

#[account]
pub struct Utxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    verifier_address_index: u64,
    blinding: u256,
    app_data_hash: u256,
    account_shielded_public_key: u256,
    account_encryption_public_key: [u8; 32],
    // // app data hash does not need to be saved because we can recalculate it from subsequent information
    test_input1: u256,
    test_input2: u256,
}
#[account]
pub struct UtxoAppData {
    test_input1: u256,
    test_input2: u256,
}
