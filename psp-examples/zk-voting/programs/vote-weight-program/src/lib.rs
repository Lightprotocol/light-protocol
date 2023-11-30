use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use light_psp4in4out_app_storage::Psp4In4OutAppStorageVerifierState;

pub mod psp_accounts;
pub use psp_accounts::*;
pub mod auto_generated_accounts;
pub use auto_generated_accounts::*;
pub mod processor;
pub use processor::*;
pub mod verifying_key_create_vote_utxo;
use light_macros::pubkey;
use light_verifier_sdk::light_transaction::Proof;
use light_verifier_sdk::{light_app_transaction::AppTransaction, light_transaction::Config};
pub use verifying_key_create_vote_utxo::*;

#[derive(Clone)]
pub struct TransactionsConfig;
impl Config for TransactionsConfig {
    /// ProgramId.
    const ID: Pubkey = pubkey!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
}
declare_id!("HJiz6qx8FQm5cKAsiEDt3HbRdv6z825PpqSYNUF9hsCx");
// [
//   150, 211,  57, 175,  10, 104, 218,  47,  33, 169,  38,
//    28,  13, 120, 245,  34, 210, 193, 114,  79,  79, 165,
//   111,  18, 236, 163,  17,  53,  75, 132, 109, 100, 242,
//    69,  86,  43,  51, 105,  36, 231,  97, 243, 153,  42,
//   204, 133,   9,  39, 126,  33,  22, 234,  48, 193, 100,
//   250,  49, 189,   6,  71, 155, 229,  39, 125
// ]
#[constant]
pub const PROGRAM_ID: &str = "HJiz6qx8FQm5cKAsiEDt3HbRdv6z825PpqSYNUF9hsCx";

#[program]
pub mod vote_weight_program {
    use super::*;

    /// This instruction is the third step of a shielded transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn verify_create_vote_weight_proof_instruction<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, VerifyCreateVoteWeightProofInstruction<'info>>,
        proof: [u8; 256],
        instruction_checked_public_inputs: [[u8; 32]; 20],
    ) -> Result<()> {
        let vote_weight_config = ctx.accounts.vote_weight_config.load()?;
        const NR_CHECKED_INPUTS: usize = VERIFYINGKEY_CREATE_VOTE_UTXO.nr_pubinputs;
        msg!("verify_create_vote_weight_proof_instruction here");
        check_current_slot(
            be_u64_from_public_input(&instruction_checked_public_inputs[0]),
            200,
        )?;

        let mut checked_public_inputs: [[u8; 32]; NR_CHECKED_INPUTS] =
            [[0u8; 32]; NR_CHECKED_INPUTS];
        // hashed and truncated program id (standardized psp public input)
        checked_public_inputs[0] = instruction_checked_public_inputs[0];
        // transaction hash (standardized psp public input)
        checked_public_inputs[1] = instruction_checked_public_inputs[1];
        // publicCurrentSlot
        checked_public_inputs[2] = instruction_checked_public_inputs[2];
        // publicCurrentSlot
        checked_public_inputs[3] = instruction_checked_public_inputs[3];
        // publicMaxLockTime
        checked_public_inputs[4] = be_u64_to_public_input(&vote_weight_config.max_lock_time);
        // publicGoverningTokenMint
        checked_public_inputs[5] = vote_weight_config.governance_token_mint.key().to_bytes();
        // publicVoteUtxoNumber
        checked_public_inputs[6] =
            be_u64_to_public_input(&vote_weight_config.current_vote_weight_number);

        let proof_app = Proof {
            a: proof[0..64].try_into().unwrap(),
            b: proof[64..192].try_into().unwrap(),
            c: proof[192..256].try_into().unwrap(),
        };

        let mut app_verifier = AppTransaction::<NR_CHECKED_INPUTS, TransactionsConfig>::new(
            &proof_app,
            &checked_public_inputs,
            &VERIFYINGKEY_CREATE_VOTE_UTXO,
        );

        app_verifier.verify()
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<
            'a,
            'b,
            'c,
            'info,
            CloseVerifierState<'info, { VERIFYINGKEY_CREATE_VOTE_UTXO.nr_pubinputs }>,
        >,
    ) -> Result<()> {
        Ok(())
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn init_vote_weight_config<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitVoteWeightConfig<'info>>,
        max_lock_time: u64,
    ) -> Result<()> {
        let mut vote_weight_config = ctx.accounts.vote_weight_config.load_init()?;
        vote_weight_config.authority = *ctx.accounts.signing_address.key;
        vote_weight_config.governance_token_mint = *ctx.accounts.governance_token_mint.key;
        vote_weight_config.max_lock_time = max_lock_time;
        Ok(())
    }

    pub fn change_config<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, ChangeConfig<'info>>,
        authority: Pubkey,
        max_lock_time: u64,
    ) -> Result<()> {
        let mut vote_weight_config = ctx.accounts.vote_weight_config.load_mut()?;
        vote_weight_config.authority = authority;
        vote_weight_config.max_lock_time = max_lock_time;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct VerifyCreateVoteWeightProofInstruction<'info> {
    /// CHECK: add check
    pub vote_weight_config: AccountLoader<'info, VoteWeightConfig>,
}

pub fn be_u64_to_public_input(input: &u64) -> [u8; 32] {
    let mut arr = [0u8; 32];
    arr[24..].copy_from_slice(&input.to_be_bytes());
    arr
}

#[inline(never)]
fn check_current_slot<'a, 'b, 'c, 'info>(slot: u64, buffer_period: u64) -> Result<()> {
    let current_slot = Clock::get()?.slot;
    if current_slot > slot + buffer_period {
        msg!(
            "Slot {} current slot {} buffer {} slot + buffer {}",
            slot,
            current_slot,
            buffer_period,
            slot + buffer_period
        );
        panic!("Slot expired, it's outside of buffer margin.");
    }
    Ok(())
}

pub fn be_u64_from_public_input(input: &[u8; 32]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&input[24..32]);
    u64::from_be_bytes(arr)
}
