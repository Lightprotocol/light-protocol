#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_compressed_token::process_transfer::InputTokenDataWithContext;
use light_compressed_token::process_transfer::PackedTokenTransferOutputData;
use light_system_program::invoke::processor::CompressedProof;
pub mod escrow_with_compressed_pda;
pub mod escrow_with_pda;

pub use escrow_with_compressed_pda::escrow::*;
pub use escrow_with_pda::escrow::*;
use light_system_program::sdk::CompressedCpiContext;
use light_system_program::NewAddressParamsPacked;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is locked")]
    EscrowLocked,
    #[msg("CpiContextAccountIndexNotFound")]
    CpiContextAccountIndexNotFound,
}

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod token_escrow {

    use self::{
        escrow_with_compressed_pda::withdrawal::process_withdraw_compressed_tokens_with_compressed_pda,
        escrow_with_pda::withdrawal::process_withdraw_compressed_escrow_tokens_with_pda,
    };

    use super::*;

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn escrow_compressed_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        lock_up_time: u64,
        escrow_amount: u64,
        proof: CompressedProof,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
    ) -> Result<()> {
        process_escrow_compressed_tokens_with_pda(
            ctx,
            lock_up_time,
            escrow_amount,
            proof,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
        )
    }

    /// Allows the owner to withdraw compressed tokens from the escrow account,
    /// provided the lockup time has expired.
    pub fn withdraw_compressed_escrow_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        bump: u8,
        withdrawal_amount: u64,
        proof: CompressedProof,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
    ) -> Result<()> {
        process_withdraw_compressed_escrow_tokens_with_pda(
            ctx,
            bump,
            withdrawal_amount,
            proof,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
        )
    }

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn escrow_compressed_tokens_with_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
        lock_up_time: u64,
        escrow_amount: u64,
        proof: CompressedProof,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        new_address_params: NewAddressParamsPacked,
        cpi_context: CompressedCpiContext,
        bump: u8,
    ) -> Result<()> {
        process_escrow_compressed_tokens_with_compressed_pda(
            ctx,
            lock_up_time,
            escrow_amount,
            proof,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            new_address_params,
            cpi_context,
            bump,
        )
    }

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn withdraw_compressed_tokens_with_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
        withdrawal_amount: u64,
        proof: CompressedProof,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        cpi_context: CompressedCpiContext,
        input_compressed_pda: PackedInputCompressedPda,
        bump: u8,
    ) -> Result<()> {
        process_withdraw_compressed_tokens_with_compressed_pda(
            ctx,
            withdrawal_amount,
            proof,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            cpi_context,
            input_compressed_pda,
            bump,
        )
    }
}

// TODO: add to light_sdk
/// A helper function that creates a new compressed account with the change output.
/// Input sum - Output sum = Change amount
/// Outputs compressed account with the change amount, and owner of the compressed input accounts.
fn create_change_output_compressed_token_account(
    input_token_data_with_context: &[InputTokenDataWithContext],
    output_compressed_accounts: &[PackedTokenTransferOutputData],
    owner: &Pubkey,
    merkle_tree_index: u8,
) -> PackedTokenTransferOutputData {
    let input_sum = input_token_data_with_context
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let output_sum = output_compressed_accounts
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let change_amount = input_sum - output_sum;
    PackedTokenTransferOutputData {
        amount: change_amount,
        owner: *owner,
        lamports: None,
        merkle_tree_index,
        tlv: None,
    }
}
