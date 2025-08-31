mod traits;
mod v1;
#[cfg(feature = "v2")]
mod v2;

// Re-export traits
// Re-export v1
pub use light_compressed_account::instruction_data::invoke_cpi::InstructionDataInvokeCpi;
// Re-export the helper function for v2
#[cfg(feature = "v2")]
pub use traits::inner_invoke_write_to_cpi_context_typed;
pub use traits::{
    invoke_light_system_program, CpiAccountsTrait, InvokeLightSystemProgram, LightCpiInstruction,
    LightInstructionData,
};
pub use v1::LightSystemProgramCpiV1;
// Re-export v2 when feature is enabled
#[cfg(feature = "v2")]
pub use v2::*;
