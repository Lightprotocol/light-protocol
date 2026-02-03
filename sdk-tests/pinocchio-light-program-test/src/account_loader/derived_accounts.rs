use light_account_pinocchio::{
    prepare_compressed_account_on_init, CpiAccounts, CpiAccountsConfig, LightFinalize,
    LightPreInit, LightSdkTypesError, PackedAddressTreeInfoExt,
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateZeroCopyRecord, CreateZeroCopyRecordParams};
use crate::state::ZeroCopyRecord;

impl LightPreInit<AccountInfo, CreateZeroCopyRecordParams> for CreateZeroCopyRecord<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateZeroCopyRecordParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            use light_account_pinocchio::{InvokeLightSystemProgram, LightAccount, LightConfig};
            use pinocchio::sysvars::{clock::Clock, Sysvar};

            let system_accounts_offset =
                params.create_accounts_proof.system_accounts_offset as usize;
            if remaining_accounts.len() < system_accounts_offset {
                return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
            }
            let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
            let cpi_accounts = CpiAccounts::new_with_config(
                self.fee_payer,
                &remaining_accounts[system_accounts_offset..],
                config,
            );

            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let output_tree_index = params.create_accounts_proof.output_state_tree_index;
            let current_account_index: u8 = 0;
            const WITH_CPI_CONTEXT: bool = false;
            let cpi_context = CompressedCpiContext::default();
            const NUM_LIGHT_PDAS: usize = 1;
            let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
            let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

            let light_config = LightConfig::load_checked(self.compression_config, &crate::ID)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let current_slot = Clock::get()
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
                .slot;

            let record_key = *self.record.key();
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

            // Set compression_info on the zero-copy record via bytemuck
            {
                let mut account_data = self
                    .record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let record_bytes =
                    &mut account_data[8..8 + core::mem::size_of::<ZeroCopyRecord>()];
                let record: &mut ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
                record.set_decompressed(&light_config, current_slot);
            }

            let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: crate::LIGHT_CPI_SIGNER.bump,
                invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: WITH_CPI_CONTEXT,
                with_transaction_hash: false,
                cpi_context,
                proof: params.create_accounts_proof.proof.0,
                new_address_params,
                account_infos,
                read_only_addresses: vec![],
                read_only_accounts: vec![],
            };

            instruction_data.invoke(cpi_accounts)?;
            Ok(false)
        };
        inner()
    }
}

impl LightFinalize<AccountInfo, CreateZeroCopyRecordParams> for CreateZeroCopyRecord<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateZeroCopyRecordParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
