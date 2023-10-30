use crate::verifying_key_swaps::VERIFYINGKEY_SWAPS;
use crate::LightInstructionThird;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::light_transaction::Proof;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::{light_app_transaction::AppTransaction, light_transaction::Config};

#[derive(Clone)]
pub struct TransactionsConfig;
impl Config for TransactionsConfig {
    const ID: Pubkey = pubkey!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
}

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

pub fn verify_program_proof<'a, 'b, 'c, 'info, const NR_CHECKED_INPUTS: usize>(
    ctx: &'a Context<'a, 'b, 'c, 'info, LightInstructionThird<'info, NR_CHECKED_INPUTS>>,
    inputs: &'a Vec<u8>,
) -> Result<()> {
    let proof_app = Proof {
        a: inputs[0..64].try_into().unwrap(),
        b: inputs[64..192].try_into().unwrap(),
        c: inputs[192..256].try_into().unwrap(),
    };
    let verifier_state = ctx.accounts.verifier_state.load()?;
    const NR_CHECKED_INPUTS: usize = VERIFYINGKEY_SWAPS.nr_pubinputs;
    let mut app_verifier = AppTransaction::<NR_CHECKED_INPUTS, TransactionsConfig>::new(
        &proof_app,
        &verifier_state.checked_public_inputs,
        &VERIFYINGKEY_SWAPS,
    );

    app_verifier.verify()
}
