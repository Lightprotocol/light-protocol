/// Account structure for mint action CPI write operations - follows the same pattern as CpiContextWriteAccounts
#[derive(Clone, Debug)]
pub struct MintActionCpiWriteAccounts<'a, T: AccountInfoTrait + Clone> {
    pub light_system_program: &'a T,
    pub mint_signer: Option<&'a T>, // Optional - only when creating mint and when creating SPL mint
    pub authority: &'a T,
    pub cpi_authority_pda: &'a T,
    pub cpi_context: &'a T,
}

impl<'a, T: AccountInfoTrait + Clone> MintActionCpiWriteAccounts<'a, T> {
    pub fn to_account_infos(&self) -> Vec<T> {
        let mut accounts = Vec::new();

        // light_system_program (always required)
        accounts.push(self.light_system_program.clone());

        // mint_signer (optional - only when creating mint and creating SPL mint)
        if let Some(mint_signer) = &self.mint_signer {
            accounts.push((*mint_signer).clone());
        }

        // authority (signer)
        accounts.push(self.authority.clone());

        // cpi_authority_pda
        accounts.push(self.cpi_authority_pda.clone());

        // cpi_context
        accounts.push(self.cpi_context.clone());

        accounts
    }
}
