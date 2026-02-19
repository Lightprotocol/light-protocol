use borsh::BorshDeserialize;
use light_account_pinocchio::{
    prepare_compressed_account_on_init, CompressionInfo, CompressedCpiContext, CpiAccounts,
    CpiAccountsConfig, InstructionDataInvokeCpiWithAccountInfo, InvokeLightSystemProgram,
    LightConfig, LightDiscriminator, LightSdkTypesError, PackedAddressTreeInfoExt,
};
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};

use super::accounts::{CreateOneByteRecord, CreateOneByteRecordParams};
use crate::state::OneByteRecord;

pub fn process(
    ctx: &CreateOneByteRecord<'_>,
    params: &CreateOneByteRecordParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
    if remaining_accounts.len() < system_accounts_offset {
        return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
    }
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        ctx.fee_payer,
        &remaining_accounts[system_accounts_offset..],
        config,
    );

    let address_tree_info = &params.create_accounts_proof.address_tree_info;
    let address_tree_pubkey = address_tree_info
        .get_tree_pubkey(&cpi_accounts)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let output_tree_index = params.create_accounts_proof.output_state_tree_index;
    let current_account_index: u8 = 0;
    let cpi_context = CompressedCpiContext::default();
    let mut new_address_params = Vec::with_capacity(1);
    let mut account_infos = Vec::with_capacity(1);

    let light_config = LightConfig::load_checked(ctx.compression_config, &crate::ID)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let current_slot = Clock::get()
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
        .slot;

    let record_key = *ctx.record.key();
    prepare_compressed_account_on_init(
        &record_key,
        &address_tree_pubkey,
        address_tree_info,
        output_tree_index,
        current_account_index,
        &crate::ID,
        &mut new_address_params,
        &mut account_infos,
    )?;

    // Set owner and compression_info on the record at offset data[disc_len..]
    {
        let disc_len = OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
        let mut account_data = ctx
            .record
            .try_borrow_mut_data()
            .map_err(|_| LightSdkTypesError::Borsh)?;
        let mut record = OneByteRecord::try_from_slice(&account_data[disc_len..])
            .map_err(|_| LightSdkTypesError::Borsh)?;
        record.owner = params.owner;
        record.compression_info = CompressionInfo::new_from_config(&light_config, current_slot);
        let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
        account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
    }

    let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1,
        bump: crate::LIGHT_CPI_SIGNER.bump,
        invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context,
        proof: params.create_accounts_proof.proof.0,
        new_address_params,
        account_infos,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    instruction_data.invoke(cpi_accounts)?;
    Ok(())
}
