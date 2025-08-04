use crate::mint_action::accounts::MintActionAccounts;
use anchor_compressed_token::ErrorCode;
use light_ctoken_types::instructions::mint_actions::ZMintActionCompressedInstructionData;

use spl_pod::solana_msg::msg;

#[derive(Debug)]
pub struct QueueIndices {
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_token_queue_index: u8,
    pub output_queue_index: u8,
}

impl QueueIndices {
    pub fn new(
        parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
        validated_accounts: &MintActionAccounts,
    ) -> Result<QueueIndices, ErrorCode> {
        let in_tree_index = parsed_instruction_data
            .cpi_context
            .as_ref()
            .map(|cpi_context| cpi_context.in_tree_index)
            .unwrap_or(1);
        let in_queue_index = parsed_instruction_data
            .cpi_context
            .as_ref()
            .map(|cpi_context| cpi_context.in_queue_index)
            .unwrap_or(2);
        let out_token_queue_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.token_out_queue_index
            } else if let Some(system_accounts) = validated_accounts.executing.as_ref() {
                if let Some(tokens_out_queue) = system_accounts.tokens_out_queue {
                    if system_accounts.out_output_queue.key() == tokens_out_queue.key() {
                        0
                    } else {
                        3
                    }
                } else {
                    0
                }
            } else {
                msg!("No system accounts provided for queue index");
                return Err(ErrorCode::MintActionMissingSystemAccountsForQueue);
            };
        let output_queue_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.out_queue_index
            } else {
                0
            };

        Ok(QueueIndices {
            in_tree_index,
            in_queue_index,
            out_token_queue_index,
            output_queue_index,
        })
    }
}
