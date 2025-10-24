use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    cpi::{
        v1::{CpiAccounts, LightSystemProgramCpi},
        CpiAccountsConfig, InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightDiscriminator, LightHasher,
};
use solana_program::{account_info::AccountInfo, msg};

use crate::ARRAY_LEN;

/// TODO: write test program with A8JgviaEAByMVLBhcebpDQ7NMuZpqBTBigC1b83imEsd (inconvenient program id)
/// CU usage:
/// - sdk pre system program cpi 10,942 CU
/// - total with V2 tree: 45,758 CU
pub fn create_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    msg!("pre instruction_data");
    let mut instruction_data = instruction_data;
    let instruction_data = CreatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;
    msg!("pre config");
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::try_new_with_config(
        &accounts[0],
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    )
    .unwrap();

    let address_tree_info = instruction_data.address_tree_info;
    let (address, address_seed) = light_sdk::address::v1::derive_address(
        &[b"compressed", instruction_data.data.as_slice()],
        &address_tree_info.get_tree_pubkey(&cpi_accounts)?,
        &crate::ID,
    );
    let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);
    msg!("pre account");
    let mut my_compressed_account = LightAccount::<MyCompressedAccount>::new_init(
        &crate::ID,
        Some(address),
        instruction_data.output_merkle_tree_index,
    );

    my_compressed_account.data = instruction_data.data;

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, instruction_data.proof)
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke(cpi_accounts)?;
    Ok(())
}

#[derive(Clone, Debug, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize)]
pub struct MyCompressedAccount {
    #[hash]
    pub data: [u8; ARRAY_LEN],
}

impl Default for MyCompressedAccount {
    fn default() -> Self {
        Self {
            data: [0u8; ARRAY_LEN],
        }
    }
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct CreatePdaInstructionData {
    pub proof: ValidityProof,
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_merkle_tree_index: u8,
    pub data: [u8; ARRAY_LEN],
    pub system_accounts_offset: u8,
    pub tree_accounts_offset: u8,
}
