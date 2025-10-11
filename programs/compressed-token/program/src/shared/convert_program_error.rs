pub fn convert_program_error(
    pinocchio_program_error: pinocchio::program_error::ProgramError,
) -> anchor_lang::prelude::ProgramError {
    anchor_lang::prelude::ProgramError::Custom(u64::from(pinocchio_program_error) as u32 + 6000)
}
