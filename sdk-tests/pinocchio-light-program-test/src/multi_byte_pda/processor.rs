use borsh::BorshDeserialize;
use light_account_pinocchio::{
    create_accounts, LightAccount, LightDiscriminator, LightSdkTypesError, PdaInitParam,
    SharedAccounts,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateMultiByteRecords, CreateMultiByteRecordsParams};
use crate::state::{
    FiveByteRecord, FourByteRecord, SevenByteRecord, SixByteRecord, ThreeByteRecord, TwoByteRecord,
};

pub fn process(
    ctx: &CreateMultiByteRecords<'_>,
    params: &CreateMultiByteRecordsParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let two_byte_record = ctx.two_byte_record;
    let three_byte_record = ctx.three_byte_record;
    let four_byte_record = ctx.four_byte_record;
    let five_byte_record = ctx.five_byte_record;
    let six_byte_record = ctx.six_byte_record;
    let seven_byte_record = ctx.seven_byte_record;
    let owner = params.owner;

    create_accounts::<AccountInfo, 6, 0, 0, 0, _>(
        [
            PdaInitParam {
                account: ctx.two_byte_record,
            },
            PdaInitParam {
                account: ctx.three_byte_record,
            },
            PdaInitParam {
                account: ctx.four_byte_record,
            },
            PdaInitParam {
                account: ctx.five_byte_record,
            },
            PdaInitParam {
                account: ctx.six_byte_record,
            },
            PdaInitParam {
                account: ctx.seven_byte_record,
            },
        ],
        |light_config, current_slot| {
            // TwoByteRecord
            {
                let disc_len = TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = two_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = TwoByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            // ThreeByteRecord
            {
                let disc_len = ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = three_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = ThreeByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            // FourByteRecord
            {
                let disc_len = FourByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = four_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = FourByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            // FiveByteRecord
            {
                let disc_len = FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = five_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = FiveByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            // SixByteRecord
            {
                let disc_len = SixByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = six_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = SixByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            // SevenByteRecord
            {
                let disc_len = SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = seven_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = SevenByteRecord::try_from_slice(&account_data[disc_len..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.owner = owner;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
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
