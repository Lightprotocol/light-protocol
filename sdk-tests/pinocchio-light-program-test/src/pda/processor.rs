use light_account_pinocchio::{
    create_accounts, LightAccount, LightSdkTypesError, PdaInitParam, SharedAccounts,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreatePda, CreatePdaParams};

pub fn process(
    ctx: &CreatePda<'_>,
    params: &CreatePdaParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let record = ctx.record;

    create_accounts::<AccountInfo, 1, 0, 0, 0, _>(
        [PdaInitParam {
            account: ctx.record,
        }],
        |light_config, current_slot| {
            use borsh::BorshDeserialize;
            let mut account_data = record
                .try_borrow_mut_data()
                .map_err(|_| LightSdkTypesError::Borsh)?;
            let mut record = crate::state::MinimalRecord::try_from_slice(&account_data[8..])
                .map_err(|_| LightSdkTypesError::Borsh)?;
            record.set_decompressed(light_config, current_slot);
            let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
            account_data[8..8 + serialized.len()].copy_from_slice(&serialized);
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
