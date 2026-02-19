use light_account_pinocchio::{
    create_accounts, LightAccount, LightSdkTypesError, PdaInitParam, SharedAccounts,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateZeroCopyRecord, CreateZeroCopyRecordParams};
use crate::state::ZeroCopyRecord;

pub fn process(
    ctx: &CreateZeroCopyRecord<'_>,
    params: &CreateZeroCopyRecordParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let record = ctx.record;

    create_accounts::<AccountInfo, 1, 0, 0, 0, _>(
        [PdaInitParam {
            account: ctx.record,
        }],
        |light_config, current_slot| {
            let mut account_data = record
                .try_borrow_mut_data()
                .map_err(|_| LightSdkTypesError::Borsh)?;
            let record_bytes = &mut account_data[8..8 + core::mem::size_of::<ZeroCopyRecord>()];
            let record: &mut ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
            record.set_decompressed(light_config, current_slot);
            Ok(())
        },
        None,
        [],
        [],
        &SharedAccounts {
            fee_payer: ctx.fee_payer,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            proof: &params.create_accounts_proof,
            program_id: crate::ID,
            compression_config: Some(ctx.compression_config),
            compressible_config: None,
            rent_sponsor: None,
            cpi_authority: None,
            system_program: None,
        },
        remaining_accounts,
    )?;
    Ok(())
}
