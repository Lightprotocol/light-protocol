use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::Discriminator;
use light_macros::pubkey;
use light_sdk::{
    account::LightAccount,
    address::derive_address,
    error::LightSdkError,
    instruction_data::LightInstructionData,
    program_merkle_context::unpack_address_merkle_context,
    system_accounts::{LightCpiAccounts, SystemAccountInfoConfig},
    verify::verify_light_accounts,
    LightDiscriminator, LightHasher,
};
use solana_program::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, program_error::ProgramError,
    pubkey::Pubkey,
};
pub const ID: Pubkey = pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    // TODO: add CreatePdaZeroCopy
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            _ => panic!("Invalid instruction discriminator."),
        }
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    let discriminator = InstructionType::try_from(instruction_data[0]).unwrap();
    match discriminator {
        InstructionType::CreatePdaBorsh => create_pda(accounts, &instruction_data[1..]),
    }?;
    Ok(())
}

pub fn create_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
    let (instruction_data, inputs) = LightInstructionData::deserialize(instruction_data)?;
    let account_data = &instruction_data[..31];

    let address_merkle_context = unpack_address_merkle_context(
        inputs.accounts.as_ref().unwrap()[0]
            .address_merkle_context
            .unwrap(),
        &accounts[9..],
    );
    solana_program::msg!(
        "create_pda address_merkle_context {:?}",
        address_merkle_context
    );
    solana_program::msg!("create_pda account_data {:?}", account_data);
    let (address, address_seed) = derive_address(
        &[b"compressed", account_data],
        &address_merkle_context,
        &crate::ID,
    );
    solana_program::msg!("create_pda address {:?}", address);
    solana_program::msg!("create_pda address_seed {:?}", address_seed);

    let account_meta = &inputs.accounts.unwrap()[0];
    let mut my_compressed_account: LightAccount<'_, MyCompressedAccount> =
        LightAccount::from_meta_init(
            account_meta,
            MyCompressedAccount::discriminator(),
            address,
            address_seed,
            &crate::ID,
        )
        .map_err(ProgramError::from)?;

    my_compressed_account.data = account_data.try_into().unwrap();

    let config = SystemAccountInfoConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        LightCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;
    solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
    verify_light_accounts(
        &light_cpi_accounts,
        inputs.proof,
        &[my_compressed_account],
        None,
        false,
        None,
    )
}

// TODO: add account traits
#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyCompressedAccount {
    data: [u8; 31],
}
