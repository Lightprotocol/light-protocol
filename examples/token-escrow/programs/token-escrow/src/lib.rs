#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use psp_compressed_pda::utils::CompressedProof;
use psp_compressed_token::InputTokenDataWithContext;
use psp_compressed_token::TokenTransferOutputData;
pub mod compressed_pda_escrow;
pub mod compressed_pda_sdk;
pub mod compressed_token_escrow;
pub mod sdk;

pub use compressed_pda_escrow::*;
pub use compressed_token_escrow::*;
use psp_compressed_pda::NewAddressParamsPacked;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is locked")]
    EscrowLocked,
}

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod token_escrow {

    use super::*;

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn escrow_compressed_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        lock_up_time: u64,
        escrow_amount: u64,
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        pubkey_array: Vec<Pubkey>,
    ) -> Result<()> {
        process_escrow_compressed_tokens_with_pda(
            ctx,
            lock_up_time,
            escrow_amount,
            proof,
            root_indices,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            pubkey_array,
        )
    }

    /// Allows the owner to withdraw compressed tokens from the escrow account,
    /// provided the lockup time has expired.
    pub fn withdraw_compressed_escrow_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        bump: u8,
        withdrawal_amount: u64,
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        pubkey_array: Vec<Pubkey>,
    ) -> Result<()> {
        process_withdraw_compressed_escrow_tokens_with_pda(
            ctx,
            bump,
            withdrawal_amount,
            proof,
            root_indices,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            pubkey_array,
        )
    }

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn escrow_compressed_tokens_with_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
        lock_up_time: u64,
        escrow_amount: u64,
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        pubkey_array: Vec<Pubkey>,
        new_address_params: NewAddressParamsPacked,
    ) -> Result<()> {
        process_escrow_compressed_tokens_with_compressed_pda(
            ctx,
            lock_up_time,
            escrow_amount,
            proof,
            root_indices,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            pubkey_array,
            new_address_params,
        )
    }
}

// TODO: add to light_sdk
/// A helper function that creates a new compressed account with the change output.
/// Input sum - Output sum = Change amount
/// Outputs compressed account with the change amount, and owner of the compressed input accounts.
fn create_change_output_compressed_token_account(
    input_token_data_with_context: &[InputTokenDataWithContext],
    output_compressed_accounts: &[TokenTransferOutputData],
    owner: &Pubkey,
) -> TokenTransferOutputData {
    let input_sum = input_token_data_with_context
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let output_sum = output_compressed_accounts
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let change_amount = input_sum - output_sum;
    TokenTransferOutputData {
        amount: change_amount,
        owner: *owner,
        lamports: None,
    }
}
