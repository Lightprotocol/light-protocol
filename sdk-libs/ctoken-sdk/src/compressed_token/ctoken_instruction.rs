use solana_instruction::Instruction;
use solana_program_error::ProgramError;

pub trait TokenInstruction: Sized {
    type ExecuteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info>;

    type CpiWriteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info>;

    fn instruction<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::ExecuteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;

    fn instruction_write_to_cpi_context_first<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;

    fn instruction_write_to_cpi_context_set<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError>;
}
