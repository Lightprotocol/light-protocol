use anchor_compressed_token::ErrorCode;
use light_ctoken_types::instructions::mint_action::ZCpiContext;
use light_program_profiler::profile;

#[derive(Debug, PartialEq)]
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
        cpi_context: Option<&ZCpiContext<'_>>,
        create_mint: bool,
        tokens_out_queue_exists: bool,
        queue_keys_match: bool,
        write_to_cpi_context: bool,
    ) -> Result<QueueIndices, ErrorCode> {
        if let Some(ctx) = cpi_context {
            // Path when cpi_context is provided
            let (in_tree_index, address_merkle_tree_index) = if create_mint {
                // if executing with cpi context address tree index must be 1.
                if !write_to_cpi_context && ctx.in_tree_index != 1 {
                    return Err(ErrorCode::MintActionInvalidCpiContextForCreateMint);
                }
                (0, ctx.in_tree_index) // in_tree_index is 0, address_merkle_tree_index from context
            } else {
                (ctx.in_tree_index, 0) // in_tree_index from context, address_merkle_tree_index is 0
            };

            Ok(QueueIndices {
                in_tree_index,
                address_merkle_tree_index,
                in_queue_index: ctx.in_queue_index,
                out_token_queue_index: ctx.token_out_queue_index,
                output_queue_index: ctx.out_queue_index,
                deduplicated: false, // not used
            })
        } else {
            // Path when cpi_context is not provided
            let (in_tree_index, address_merkle_tree_index) = if create_mint {
                (0, 1) // in_tree_index is 0, address_merkle_tree_index defaults to 1
            } else {
                (1, 0) // in_tree_index defaults to 1, address_merkle_tree_index is 0
            };

            let out_token_queue_index = if tokens_out_queue_exists {
                if queue_keys_match {
                    0 // Queue keys match - use same index
                } else {
                    3 // Queue keys don't match - use different index
                }
            } else {
                0 // No tokens queue
            };

            let output_queue_index = 0;

            let deduplicated =
                tokens_out_queue_exists && out_token_queue_index == output_queue_index;

            Ok(QueueIndices {
                in_tree_index,
                address_merkle_tree_index,
                in_queue_index: 2,
                out_token_queue_index,
                output_queue_index,
                deduplicated,
            })
        }
    }
}
