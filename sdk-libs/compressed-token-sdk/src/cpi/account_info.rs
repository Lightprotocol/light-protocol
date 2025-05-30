use crate::state::InputTokenDataWithContext;
use light_compressed_account::compressed_account::PackedMerkleContext;

/// Get an existing compressed token account from token_data in optimized
/// format.
///
/// Example:
/// ```rust
/// let data = InstructionData::try_from_slice(instruction_data)
///     .map_err(|_| ProgramError::InvalidInstructionData)?;
///
/// let compressed_token_account = get_compressed_token_account_info(
///     data.amount,
///     data.merkle_context,
///     data.root_index,
///     None
/// );
/// ```
pub fn get_compressed_token_account_info(
    merkle_context: PackedMerkleContext,
    root_index: u16,
    amount: u64,
    lamports: Option<u64>,
) -> InputTokenDataWithContext {
    InputTokenDataWithContext {
        amount,
        delegate_index: None,
        merkle_context,
        root_index,
        lamports,
        tlv: None,
    }
}
