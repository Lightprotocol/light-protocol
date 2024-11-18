use std::{io::Cursor, mem};

use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account_info::LightAccountInfo,
    account_meta::LightAccountMeta,
    address::derive_address,
    error::LightSdkError,
    hasher::Discriminator,
    instruction_data::LightInstructionData,
    program_merkle_context::unpack_address_merkle_context,
    proof::ProofRpcResult,
    verify::{verify_light_account_infos, LightCpiAccounts},
    LightDiscriminator, LightHasher,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use thiserror::Error;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::unpack(instruction_data)?;

    match instruction {
        Instruction::WithCompressedAccount { inputs, name } => {
            with_compressed_account(program_id, accounts, inputs, name)
        }
        Instruction::UpdateNestedData {
            inputs,
            nested_data,
        } => update_nested_data(program_id, accounts, inputs, nested_data),
    }
}

fn with_compressed_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    inputs: LightInstructionData,
    name: String,
) -> ProgramResult {
    let light_accounts = inputs
        .accounts
        .as_ref()
        .ok_or(LightSdkError::ExpectedAccounts)?;

    let address_merkle_context = light_accounts[0]
        .address_merkle_context
        .ok_or(LightSdkError::ExpectedAddressMerkleContext)?;
    let address_merkle_context = unpack_address_merkle_context(address_merkle_context, accounts);
    let (address, address_seed) = derive_address(
        &[b"compressed", name.as_bytes()],
        &address_merkle_context,
        program_id,
    );

    let my_compressed_account = LightAccountInfo::from_meta_init(
        &light_accounts[0],
        MyCompressedAccount::discriminator(),
        address,
        address_seed,
        Some(name.len() + mem::size_of::<NestedData>()),
        program_id,
    )?;

    let cpda = MyCompressedAccount {
        name,
        nested: NestedData::default(),
    };
    let data = my_compressed_account
        .data
        .ok_or(LightSdkError::ExpectedData)?;
    let mut data = data.borrow_mut();
    let mut write_cursor = Cursor::new(&mut *data);
    cpda.serialize(&mut write_cursor)?;

    let accounts = &mut accounts.iter();
    let fee_payer = next_account_info(accounts)?;
    let authority = next_account_info(accounts)?;
    let registered_program_pda = next_account_info(accounts)?;
    let noop_program = next_account_info(accounts)?;
    let account_compression_authority = next_account_info(accounts)?;
    let account_compression_program = next_account_info(accounts)?;
    let invoking_program = next_account_info(accounts)?;
    let light_system_program = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;

    let light_cpi_accounts = LightCpiAccounts {
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        invoking_program,
        light_system_program,
        system_program,
        cpi_context: None,
    };
    verify_light_account_infos(
        light_cpi_accounts,
        &[],
        inputs.proof,
        &[my_compressed_account],
        None,
        false,
        None,
    )?;

    Ok(())
}

fn update_nested_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    inputs: LightInstructionData,
    nested_data: NestedData,
) -> ProgramResult {
    let light_accounts = inputs
        .accounts
        .as_ref()
        .ok_or(LightSdkError::ExpectedAccounts)?;

    let my_compressed_account = LightAccountInfo::from_meta_mut(
        &light_accounts[0],
        MyCompressedAccount::discriminator(),
        program_id,
    )?;

    let data = my_compressed_account
        .data
        .ok_or(LightSdkError::ExpectedData)?;
    let mut data = data.borrow_mut();
    let mut cpda = MyCompressedAccount::deserialize(&mut data.as_slice())?;

    cpda.nested = nested_data;

    let mut write_cursor = Cursor::new(&mut *data);
    cpda.serialize(&mut write_cursor)?;

    let accounts = &mut accounts.iter();
    let fee_payer = next_account_info(accounts)?;
    let authority = next_account_info(accounts)?;
    let registered_program_pda = next_account_info(accounts)?;
    let noop_program = next_account_info(accounts)?;
    let account_compression_authority = next_account_info(accounts)?;
    let account_compression_program = next_account_info(accounts)?;
    let invoking_program = next_account_info(accounts)?;
    let light_system_program = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;

    let light_cpi_accounts = LightCpiAccounts {
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        invoking_program,
        light_system_program,
        system_program,
        cpi_context: None,
    };
    verify_light_account_infos(
        light_cpi_accounts,
        &[],
        inputs.proof,
        &[my_compressed_account],
        None,
        false,
        None,
    )?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum MyError {
    #[error("invalid instruction")]
    InvalidInstruction,
}

impl From<MyError> for ProgramError {
    fn from(e: MyError) -> Self {
        let e = match e {
            MyError::InvalidInstruction => 1,
        };
        ProgramError::Custom(e)
    }
}

pub enum Instruction {
    WithCompressedAccount {
        inputs: LightInstructionData,
        name: String,
    },
    UpdateNestedData {
        inputs: LightInstructionData,
        nested_data: NestedData,
    },
}

impl Instruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(MyError::InvalidInstruction)?;
        match tag {
            0 => {
                let mut cur = Cursor::new(rest);

                let proof = Option::<ProofRpcResult>::deserialize_reader(&mut cur)?;
                let accounts = Option::<Vec<LightAccountMeta>>::deserialize_reader(&mut cur)?;
                let inputs = LightInstructionData { proof, accounts };

                let name = String::deserialize_reader(&mut cur)?;

                Ok(Self::WithCompressedAccount { inputs, name })
            }
            1 => {
                let mut cur = Cursor::new(rest);

                let proof = Option::<ProofRpcResult>::deserialize_reader(&mut cur)?;
                let accounts = Option::<Vec<LightAccountMeta>>::deserialize_reader(&mut cur)?;
                let inputs = LightInstructionData { proof, accounts };

                let nested_data = NestedData::deserialize_reader(&mut cur)?;

                Ok(Self::UpdateNestedData {
                    inputs,
                    nested_data,
                })
            }
            _ => Err(MyError::InvalidInstruction.into()),
        }
    }
}

#[derive(
    BorshDeserialize, BorshSerialize, Clone, Debug, Default, LightDiscriminator, LightHasher,
)]
pub struct MyCompressedAccount {
    name: String,
    #[nested]
    pub nested: NestedData,
}

// Illustrates nested hashing feature.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, LightHasher)]
pub struct NestedData {
    pub one: u16,
    pub two: u16,
    pub three: u16,
    pub four: u16,
    pub five: u16,
    pub six: u16,
    pub seven: u16,
    pub eight: u16,
    pub nine: u16,
    pub ten: u16,
    pub eleven: u16,
    pub twelve: u16,
}

impl Default for NestedData {
    fn default() -> Self {
        Self {
            one: 1,
            two: 2,
            three: 3,
            four: 4,
            five: 5,
            six: 6,
            seven: 7,
            eight: 8,
            nine: 9,
            ten: 10,
            eleven: 11,
            twelve: 12,
        }
    }
}
