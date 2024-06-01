use anchor_lang::prelude::AccountInfo;

pub trait LightTraitsFeePayer<'info> {
    fn get_light_traits_fee_payer(&self) -> AccountInfo<'info>;
}
pub trait LightTraitsAuthority<'info> {
    fn get_light_traits_authority(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsRegisteredProgramPda<'info> {
    fn get_light_traits_registered_program_pda(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsNoopProgram<'info> {
    fn get_light_traits_noop_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsLightSystemProgram<'info> {
    fn get_light_traits_light_system_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsAccountCompressionAuthority<'info> {
    fn get_light_traits_account_compression_authority(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsAccountCompressionProgram<'info> {
    fn get_light_traits_account_compression_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsInvokingProgram<'info> {
    fn get_light_traits_invoking_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsSystemProgram<'info> {
    fn get_light_traits_system_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraitsCpiContextAccount<'info> {
    fn get_light_traits_cpi_context_account(&self) -> Option<AccountInfo<'info>>;
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
