use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{compressed_account::PackedMerkleContext, pubkey::Pubkey};
use light_hasher::{DataHasher, Discriminator, Poseidon};
use light_macros::pubkey;
use light_sdk::{
    account_info::LightAccountInfo,
    account_meta::InputAccountMetaWithAddress,
    address::derive_address,
    error::LightSdkError,
    instruction_data::LightInstructionData,
    program_merkle_context::unpack_address_merkle_context,
    system_accounts::{LightCpiAccounts, SystemAccountInfoConfig},
    verify::verify_light_account_infos,
    LightDiscriminator, LightHasher,
};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopy, ZeroCopyEq};
use solana_program::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, program_error::ProgramError,
};
pub const ID: solana_program::pubkey::Pubkey =
    pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

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
    _program_id: &solana_program::pubkey::Pubkey,
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

    let address_merkle_context =
        unpack_address_merkle_context(inputs.new_addresses.as_ref().unwrap()[0], &accounts[9..]);
    solana_program::msg!(
        "create_pda address_merkle_context {:?}",
        address_merkle_context
    );
    let account_data = &instruction_data[..31];
    solana_program::msg!("create_pda account_data {:?}", account_data);
    let (address, address_seed) = derive_address(
        &[b"compressed", account_data],
        &address_merkle_context,
        &crate::ID,
    );
    solana_program::msg!("create_pda address {:?}", address);
    solana_program::msg!("create_pda address_seed {:?}", address_seed);

    let my_compressed_account = MyCompressedAccount {
        signer: (*accounts[0].key).into(),
        data: account_data.try_into().unwrap(),
    };
    let account_info = LightAccountInfo::init_with_address(
        &crate::ID,
        MyCompressedAccount::discriminator(),
        my_compressed_account.try_to_vec().unwrap(),
        // TODO: make poseidon default, and change to hash_with::<GenericHasher>
        my_compressed_account.hash::<Poseidon>().unwrap(),
        address,
        0,
    );

    let config = SystemAccountInfoConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        LightCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;
    solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
    verify_light_account_infos(
        &light_cpi_accounts,
        inputs.proof,
        &[account_info],
        None,
        false,
        None,
    )
}

pub fn update_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
    let (instruction_data, inputs) = LightInstructionData::deserialize(instruction_data)?;
    let (instruction_data, _) = UpdateInstructionData::zero_copy_at(instruction_data).unwrap();
    let my_compressed_account = MyCompressedAccount {
        signer: (*accounts[0].key).into(),
        data: instruction_data.input_compressed_account.data,
    };
    let input_compressed_account_info = light_sdk::account_info::LightInputAccountInfo {
        lamports: None,
        address: instruction_data
            .input_compressed_account
            .meta
            .address
            .map(|x| *x),
        data_hash: Some(my_compressed_account.hash::<Poseidon>().unwrap()),
        merkle_context: instruction_data
            .input_compressed_account
            .meta
            .merkle_context
            .into(),
        root_index: (*instruction_data.input_compressed_account.meta.root_index).into(),
    };

    let mut account_info = LightAccountInfo::from_meta_mut(
        input_compressed_account_info,
        &crate::ID,
        my_compressed_account.try_to_vec().unwrap(),
        MyCompressedAccount::discriminator(),
        0,
    )
    .unwrap();
    // Ugly af, can be avoided by separating input and output accounts.
    let data_slice: &mut [u8] = account_info.data.as_mut().unwrap().as_mut_slice();
    let (mut my_account, _) = MyCompressedAccount::zero_copy_at_mut(data_slice).unwrap();
    my_account.data = *instruction_data.new_data;

    let config = SystemAccountInfoConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        LightCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;
    solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
    verify_light_account_infos(
        &light_cpi_accounts,
        inputs.proof,
        &[account_info],
        None,
        false,
        None,
    )
}

// TODO: add account traits
#[derive(
    Clone,
    Debug,
    Default,
    LightHasher,
    LightDiscriminator,
    BorshDeserialize,
    BorshSerialize,
    ZeroCopy,
    ZeroCopyEq,
)]
pub struct MyCompressedAccount {
    signer: Pubkey,
    data: [u8; 31],
}

#[derive(Debug, ZeroCopy)]
pub struct UpdateInstructionData {
    pub input_compressed_account: InputMyCompressedAccountWithContext,
    pub new_data: [u8; 31],
}

#[derive(Debug, ZeroCopy)]
pub struct InputMyCompressedAccountWithContext {
    pub data: [u8; 31],
    pub meta: InputAccountMetaWithAddress,
}
