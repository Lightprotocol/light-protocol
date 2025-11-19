use solana_instruction::Instruction;
use solana_program_error::ProgramError;

/// Trait for compressed token instruction types.
///
/// This trait provides a unified interface for building instructions for the
/// compressed token program. All compressed token instruction data types
/// (MintActionCompressedInstructionData, TransferCompressedInstructionData, etc.)
/// should implement this trait.
///
/// The trait separates instruction building from invocation, allowing flexibility
/// in how instructions are used (client-side vs CPI, testing, etc.).
pub trait CTokenInstruction: Sized {
    /// Account structure type for execute mode (with full accounts)
    type ExecuteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info>;

    /// Account structure type for CPI write mode (minimal accounts for batching)
    type CpiWriteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info>;

    /// Build the instruction in execute mode.
    ///
    /// This creates a full instruction with all necessary accounts for direct execution
    /// or CPI invocation with proof validation.
    ///
    /// # Arguments
    /// * `accounts` - Full account structure required for instruction execution
    ///
    /// # Returns
    /// The built instruction ready to be invoked
    fn instruction<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::ExecuteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;

    /// Build the instruction in CPI write mode as the first operation in a batch.
    ///
    /// This mode is used when batching multiple operations across different programs
    /// using CPI context. The first operation sets up the CPI context.
    ///
    /// # Arguments
    /// * `accounts` - Minimal account structure for CPI write mode
    ///
    /// # Returns
    /// The built instruction with `first_set_context = true`
    fn instruction_write_to_cpi_context_first<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;

    /// Build the instruction in CPI write mode as a subsequent operation in a batch.
    ///
    /// This mode is used for operations after the first in a CPI context batch.
    ///
    /// # Arguments
    /// * `accounts` - Minimal account structure for CPI write mode
    ///
    /// # Returns
    /// The built instruction with `set_context = true`
    fn instruction_write_to_cpi_context_set<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;
}
