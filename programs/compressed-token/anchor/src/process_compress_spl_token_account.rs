use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;

use super::TransferInstruction;
use crate::{
    process_transfer::{
        process_transfer, CompressedTokenInstructionDataTransfer, PackedTokenTransferOutputData,
    },
    ErrorCode,
};

pub fn process_compress_spl_token_account<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
    owner: Pubkey,
    remaining_amount: Option<u64>,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    crate::check_cpi_context(&cpi_context)?;
    let compression_token_account =
        if let Some(token_account) = ctx.accounts.compress_or_decompress_token_account.as_ref() {
            token_account
        } else {
            return err!(ErrorCode::CompressedPdaUndefinedForCompress);
        };
    let compress_amount = compression_token_account
        .amount
        .checked_sub(remaining_amount.unwrap_or_default())
        .ok_or(crate::ErrorCode::InsufficientTokenAccountBalance)?;
    let compressed_output_account = PackedTokenTransferOutputData {
        owner,
        lamports: None,
        amount: compress_amount,
        tlv: None,
        merkle_tree_index: 0,
    };

    let inputs = CompressedTokenInstructionDataTransfer {
        proof: None,
        mint: compression_token_account.mint,
        delegated_transfer: None,
        is_compress: true,
        input_token_data_with_context: Vec::new(),
        output_compressed_accounts: vec![compressed_output_account],
        cpi_context,
        lamports_change_account_merkle_tree_index: None,
        compress_or_decompress_amount: Some(compress_amount),
        with_transaction_hash: false,
    };
    process_transfer(ctx, inputs)
}

#[cfg(not(target_os = "solana"))]
pub mod sdk {
    use anchor_lang::{prelude::AccountMeta, InstructionData, ToAccountMetas};
    use anchor_spl::token::ID as TokenProgramId;
    use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::get_token_pool_pda_with_index;

    #[allow(clippy::too_many_arguments)]
    pub fn create_compress_spl_token_account_instruction(
        owner: &Pubkey,
        remaining_amount: Option<u64>,
        cpi_context: Option<CompressedCpiContext>,
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
        output_merkle_tree: &Pubkey,
        token_account: &Pubkey,
        is_token_22: bool,
        token_pool_index: u8,
    ) -> Instruction {
        let instruction_data = crate::instruction::CompressSplTokenAccount {
            owner: *owner,
            remaining_amount,
            cpi_context,
        };
        let (cpi_authority_pda, _) = crate::process_transfer::get_cpi_authority_pda();
        let token_pool_pda = get_token_pool_pda_with_index(mint, token_pool_index);
        let token_program = if is_token_22 {
            Some(anchor_spl::token_2022::ID)
        } else {
            Some(TokenProgramId)
        };

        let accounts = crate::accounts::TransferInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            cpi_authority_pda,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(*token_account),
            token_program,
            system_program: solana_sdk::system_program::ID,
        };
        let remaining_accounts = vec![AccountMeta::new(*output_merkle_tree, false)];

        Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
            data: instruction_data.data(),
        }
    }
}
