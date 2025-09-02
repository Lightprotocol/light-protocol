use anchor_lang::prelude::ProgramError;
use light_ctoken_types::instructions::transfer2::ZCompressedTokenInstructionDataTransfer2;

/// Configuration for Transfer2 account validation
/// Replaces complex boolean parameters with clean single config object
/// Follows mint_action AccountsConfig pattern
#[derive(Debug)]
pub struct Transfer2Config {
    /// SOL token pool required for lamport imbalance.
    pub sol_pool_required: bool,
    /// SOL decompression recipient required.
    pub sol_decompression_required: bool,
    /// CPI context operations required.
    pub cpi_context_required: bool,
    /// CPI context write operations required.
    pub cpi_context_write_required: bool,
    /// Total input lamports (checked arithmetic).
    pub total_input_lamports: u64,
    /// Total output lamports (checked arithmetic).
    pub total_output_lamports: u64,
    pub no_compressed_accounts: bool,
}

impl Transfer2Config {
    /// Create configuration from instruction data
    /// Centralizes the boolean logic that was previously scattered in processor
    pub fn from_instruction_data(
        inputs: &ZCompressedTokenInstructionDataTransfer2,
    ) -> Result<Self, ProgramError> {
        let (input_lamports, output_lamports) = Self::calculate_lamport_totals(inputs)?;
        let no_compressed_accounts =
            inputs.in_token_data.is_empty() && inputs.out_token_data.is_empty();
        Ok(Self {
            sol_pool_required: input_lamports != output_lamports,
            sol_decompression_required: input_lamports < output_lamports,
            cpi_context_required: inputs.cpi_context.is_some(),
            cpi_context_write_required: inputs
                .cpi_context
                .as_ref()
                .map(|x| x.first_set_context || x.set_context)
                .unwrap_or_default(),
            total_input_lamports: input_lamports,
            total_output_lamports: output_lamports,
            no_compressed_accounts,
        })
    }

    /// Calculate total input and output lamports from instruction data
    /// Returns error on arithmetic overflow for security
    fn calculate_lamport_totals(
        inputs: &ZCompressedTokenInstructionDataTransfer2,
    ) -> Result<(u64, u64), ProgramError> {
        let input_lamports = if let Some(in_lamports) = inputs.in_lamports.as_ref() {
            in_lamports
                .iter()
                .try_fold(0u64, |acc, input| acc.checked_add(u64::from(**input)))
                .ok_or(ProgramError::ArithmeticOverflow)?
        } else {
            0
        };

        let output_lamports = if let Some(out_lamports) = inputs.out_lamports.as_ref() {
            out_lamports
                .iter()
                .try_fold(0u64, |acc, output| acc.checked_add(u64::from(**output)))
                .ok_or(ProgramError::ArithmeticOverflow)?
        } else {
            0
        };

        Ok((input_lamports, output_lamports))
    }
}
