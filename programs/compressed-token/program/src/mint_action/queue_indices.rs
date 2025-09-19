use anchor_compressed_token::ErrorCode;
use light_ctoken_types::instructions::mint_action::ZMintActionCompressedInstructionData;
use light_profiler::profile;
use spl_pod::solana_msg::msg;

use crate::mint_action::accounts::MintActionAccounts;

#[derive(Debug)]
pub struct QueueIndices {
    pub in_tree_index: u8,
    pub address_merkle_tree_index: u8,
    pub in_queue_index: u8,
    pub out_token_queue_index: u8,
    pub output_queue_index: u8,
    pub deduplicated: bool,
}

impl QueueIndices {
    #[profile]
    pub fn new(
        parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
        validated_accounts: &MintActionAccounts,
    ) -> Result<QueueIndices, ErrorCode> {
        // For create mint, in_tree_index points to address merkle tree
        // For existing mint, in_tree_index points to in merkle tree
        let (in_tree_index, address_merkle_tree_index) = if parsed_instruction_data.create_mint() {
            // When creating mint, the in_tree_index actually refers to the address tree
            let address_tree_idx = parsed_instruction_data
                .cpi_context
                .as_ref()
                .map(|cpi_context| cpi_context.in_tree_index)
                .unwrap_or(1);
            (0, address_tree_idx) // in_tree_index is 0 when not used
        } else {
            // When mint exists, in_tree_index is for the state merkle tree
            let in_tree_idx = parsed_instruction_data
                .cpi_context
                .as_ref()
                .map(|cpi_context| cpi_context.in_tree_index)
                .unwrap_or(1);
            (in_tree_idx, 0) // address_merkle_tree_index is 0 when not used
        };

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
                    let out_queue_key = system_accounts.out_output_queue.key();
                    let tokens_queue_key = tokens_out_queue.key();
                    if out_queue_key == tokens_queue_key {
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

        let tokens_outqueue_exists = validated_accounts
            .executing
            .as_ref()
            .map(|executing| executing.tokens_out_queue.is_some())
            .unwrap_or(false);
        let deduplicated = tokens_outqueue_exists && out_token_queue_index == output_queue_index;
        Ok(QueueIndices {
            in_tree_index,
            address_merkle_tree_index,
            in_queue_index,
            out_token_queue_index,
            output_queue_index,
            deduplicated,
        })
    }
}
