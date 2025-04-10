use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    hashv_to_bn254_field_size_be, instruction_data::data::NewAddressParamsPacked,
};
use light_sdk::{
    account::CBorshAccount,
    cpi::{
        accounts::{CompressionCpiAccounts, CompressionCpiAccountsConfig},
        verify::verify_compressed_account_infos,
    },
    error::LightSdkError,
    instruction::{
        instruction_data::LightInstructionData, merkle_context::unpack_address_merkle_context,
    },
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;

/// CU usage:
/// - sdk pre system program cpi 10,942 CU
/// - total with V1 tree: 307,784 CU
/// - total with V2 tree: 181,932 CU
pub fn create_pda<const BATCHED: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreatePdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    let address_merkle_context = unpack_address_merkle_context(
        instruction_data
            .light_ix_data
            .new_addresses
            .as_ref()
            .unwrap()[0],
        &accounts[9..],
    );

    let (address, address_seed) = if BATCHED {
        let address_seed =
            hashv_to_bn254_field_size_be(&[b"compressed", instruction_data.data.as_slice()]);
        let address = light_compressed_account::address::derive_address(
            &address_seed,
            &address_merkle_context.address_merkle_tree_pubkey.to_bytes(),
            &crate::ID.to_bytes(),
        );
        (address, address_seed)
    } else {
        light_sdk::address::v1::derive_address(
            &[b"compressed", instruction_data.data.as_slice()],
            &address_merkle_context,
            &crate::ID,
        )
    };
    let new_address_params = NewAddressParamsPacked {
        seed: address_seed,
        address_queue_account_index: instruction_data
            .light_ix_data
            .new_addresses
            .as_ref()
            .unwrap()[0]
            .address_queue_pubkey_index,
        address_merkle_tree_root_index: instruction_data
            .light_ix_data
            .new_addresses
            .as_ref()
            .unwrap()[0]
            .root_index,
        address_merkle_tree_account_index: instruction_data
            .light_ix_data
            .new_addresses
            .as_ref()
            .unwrap()[0]
            .address_merkle_tree_pubkey_index,
    };

    let program_id = crate::ID.into();
    let mut my_compressed_account = CBorshAccount::<'_, MyCompressedAccount>::new_init(
        &program_id,
        Some(address),
        instruction_data.output_merkle_tree_index,
    );

    my_compressed_account.data = instruction_data.data;

    let config = CompressionCpiAccountsConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        CompressionCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;

    verify_compressed_account_infos(
        &light_cpi_accounts,
        instruction_data.light_ix_data.proof,
        &[my_compressed_account.to_account_info()?],
        Some(vec![new_address_params]),
        None,
        false,
        None,
    )
}

#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyCompressedAccount {
    pub data: [u8; 31],
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreatePdaInstructionData {
    pub light_ix_data: LightInstructionData,
    pub output_merkle_tree_index: u8,
    pub data: [u8; 31],
}
