use light_account_pinocchio::{
    create_accounts, AtaInitParam, CreateMintsInput, LightAccount, LightSdkTypesError,
    PdaInitParam, SharedAccounts, SingleMintParams, TokenInitParam,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAllAccounts, CreateAllParams};

pub fn process(
    ctx: &CreateAllAccounts<'_>,
    params: &CreateAllParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    use borsh::BorshDeserialize;

    const NUM_LIGHT_PDAS: usize = 3;
    const NUM_LIGHT_MINTS: usize = 1;
    const NUM_TOKENS: usize = 1;
    const NUM_ATAS: usize = 1;

    let authority_key = *ctx.authority.key();
    let mint_signer_key = *ctx.mint_signer.key();
    let mint_key = *ctx.mint.key();

    let mint_signer_seeds: &[&[u8]] = &[
        crate::MINT_SIGNER_SEED_A,
        authority_key.as_ref(),
        &[params.mint_signer_bump],
    ];

    let vault_seeds: &[&[u8]] = &[
        crate::VAULT_SEED,
        mint_key.as_ref(),
        &[params.token_vault_bump],
    ];

    let borsh_record = ctx.borsh_record;
    let zero_copy_record = ctx.zero_copy_record;
    let one_byte_record = ctx.one_byte_record;

    create_accounts::<AccountInfo, NUM_LIGHT_PDAS, NUM_LIGHT_MINTS, NUM_TOKENS, NUM_ATAS, _>(
        [
            PdaInitParam {
                account: ctx.borsh_record,
            },
            PdaInitParam {
                account: ctx.zero_copy_record,
            },
            PdaInitParam {
                account: ctx.one_byte_record,
            },
        ],
        |light_config, current_slot| {
            // Set compression_info on the Borsh record
            {
                let mut account_data = borsh_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record = crate::state::MinimalRecord::try_from_slice(&account_data[8..])
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[8..8 + serialized.len()].copy_from_slice(&serialized);
            }
            // Set compression_info on the ZeroCopy record
            {
                let mut account_data = zero_copy_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let record_bytes =
                    &mut account_data[8..8 + core::mem::size_of::<crate::state::ZeroCopyRecord>()];
                let record: &mut crate::state::ZeroCopyRecord =
                    bytemuck::from_bytes_mut(record_bytes);
                record.set_decompressed(light_config, current_slot);
            }
            // Set compression_info on the OneByteRecord
            {
                use light_account_pinocchio::LightDiscriminator;
                let disc_len = crate::state::OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
                let mut account_data = one_byte_record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let mut record =
                    crate::state::OneByteRecord::try_from_slice(&account_data[disc_len..])
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                record.set_decompressed(light_config, current_slot);
                let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                account_data[disc_len..disc_len + serialized.len()].copy_from_slice(&serialized);
            }
            Ok(())
        },
        Some(CreateMintsInput {
            params: [SingleMintParams {
                decimals: 9,
                mint_authority: authority_key,
                mint_bump: None,
                freeze_authority: None,
                mint_seed_pubkey: mint_signer_key,
                authority_seeds: None,
                mint_signer_seeds: Some(mint_signer_seeds),
                token_metadata: None,
            }],
            mint_seed_accounts: [ctx.mint_signers_slice[0]],
            mint_accounts: [ctx.mints_slice[0]],
        }),
        [TokenInitParam {
            account: ctx.token_vault,
            mint: ctx.mint,
            owner: *ctx.vault_owner.key(),
            seeds: vault_seeds,
        }],
        [AtaInitParam {
            ata: ctx.user_ata,
            owner: ctx.ata_owner,
            mint: ctx.mint,
            idempotent: false,
        }],
        &SharedAccounts {
            fee_payer: ctx.payer,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            proof: &params.create_accounts_proof,
            program_id: crate::ID,
            compression_config: Some(ctx.compression_config),
            compressible_config: Some(ctx.compressible_config),
            rent_sponsor: Some(ctx.rent_sponsor),
            cpi_authority: Some(ctx.cpi_authority),
            system_program: Some(ctx.system_program),
        },
        remaining_accounts,
    )?;
    Ok(())
}
