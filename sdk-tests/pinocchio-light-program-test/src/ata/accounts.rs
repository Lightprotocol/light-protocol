use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct CreateAtaParams {}

pub struct CreateAtaAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub ata_owner: &'a AccountInfo,
    pub user_ata: &'a AccountInfo,
    pub compressible_config: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub light_token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateAtaAccounts<'a> {
    pub const FIXED_LEN: usize = 8;

    pub fn parse(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        let payer = &accounts[0];
        let mint = &accounts[1];
        let ata_owner = &accounts[2];
        let user_ata = &accounts[3];
        let compressible_config = &accounts[4];
        let rent_sponsor = &accounts[5];
        let light_token_program = &accounts[6];
        let system_program = &accounts[7];

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            payer,
            mint,
            ata_owner,
            user_ata,
            compressible_config,
            rent_sponsor,
            light_token_program,
            system_program,
        })
    }
}
