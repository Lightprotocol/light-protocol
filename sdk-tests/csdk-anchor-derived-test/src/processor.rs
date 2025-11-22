use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_compressed_token_sdk::compat::PackedCTokenData;
use light_sdk::{
    compressible::{compress_account::prepare_account_for_compression, CompressibleConfig},
    cpi::v2::CpiAccounts,
    instruction::{account_meta::CompressedAccountMetaNoLamportsNoAddress, ValidityProof},
    LightDiscriminator,
};

use crate::{
    instruction_accounts::{CompressAccountsIdempotent, DecompressAccountsIdempotent},
    state::{GameSession, PlaceholderRecord, UserRecord},
    variant::{CTokenAccountVariant, CompressedAccountData, CompressedAccountVariant},
    LIGHT_CPI_SIGNER,
};

impl light_sdk::compressible::HasTokenVariant for CompressedAccountData {
    fn is_packed_ctoken(&self) -> bool {
        matches!(self.data, CompressedAccountVariant::PackedCTokenData(_))
    }
}

/// Empty struct since this test doesn't use data.* fields in PDA seeds
#[derive(Default)]
pub struct SeedParams;

impl<'info> light_sdk::compressible::DecompressContext<'info>
    for DecompressAccountsIdempotent<'info>
{
    type CompressedData = CompressedAccountData;
    type PackedTokenData = PackedCTokenData<CTokenAccountVariant>;
    type CompressedMeta = CompressedAccountMetaNoLamportsNoAddress;
    type SeedParams = SeedParams;

    fn fee_payer(&self) -> &AccountInfo<'info> {
        self.fee_payer.as_ref()
    }

    fn config(&self) -> &AccountInfo<'info> {
        &self.config
    }

    fn rent_sponsor(&self) -> &AccountInfo<'info> {
        self.rent_sponsor.as_ref()
    }

    fn ctoken_rent_sponsor(&self) -> Option<&AccountInfo<'info>> {
        self.ctoken_rent_sponsor.as_ref()
    }

    fn ctoken_program(&self) -> Option<&AccountInfo<'info>> {
        self.ctoken_program.as_ref()
    }

    fn ctoken_cpi_authority(&self) -> Option<&AccountInfo<'info>> {
        self.ctoken_cpi_authority.as_ref()
    }

    fn ctoken_config(&self) -> Option<&AccountInfo<'info>> {
        self.ctoken_config.as_ref()
    }

    fn collect_pda_and_token<'b>(
        &self,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        address_space: Pubkey,
        compressed_accounts: Vec<Self::CompressedData>,
        solana_accounts: &[AccountInfo<'info>],
        _seed_params: Option<&Self::SeedParams>,
    ) -> std::result::Result<
        (
            Vec<CompressedAccountInfo>,
            Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
        ),
        ProgramError,
    > {
        let post_system_offset = cpi_accounts.system_accounts_end_offset();
        let all_infos = cpi_accounts.account_infos();
        let post_system_accounts = &all_infos[post_system_offset..];

        let mut compressed_pda_infos = Vec::new();
        let mut compressed_token_accounts = Vec::new();

        for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
            let meta = compressed_data.meta;
            match compressed_data.data {
                CompressedAccountVariant::PackedUserRecord(packed) => {
                    light_sdk::compressible::handle_packed_pda_variant::<
                        UserRecord,
                        _,
                        DecompressAccountsIdempotent<'info>,
                        SeedParams,
                    >(
                        self.rent_sponsor.as_ref(),
                        cpi_accounts,
                        address_space,
                        &solana_accounts[i],
                        i,
                        &packed,
                        &meta,
                        post_system_accounts,
                        &mut compressed_pda_infos,
                        &crate::ID,
                        self,
                        None,
                    )?;
                }
                CompressedAccountVariant::PackedGameSession(packed) => {
                    light_sdk::compressible::handle_packed_pda_variant::<
                        GameSession,
                        _,
                        DecompressAccountsIdempotent<'info>,
                        SeedParams,
                    >(
                        self.rent_sponsor.as_ref(),
                        cpi_accounts,
                        address_space,
                        &solana_accounts[i],
                        i,
                        &packed,
                        &meta,
                        post_system_accounts,
                        &mut compressed_pda_infos,
                        &crate::ID,
                        self,
                        None,
                    )?;
                }
                CompressedAccountVariant::PackedPlaceholderRecord(packed) => {
                    light_sdk::compressible::handle_packed_pda_variant::<
                        PlaceholderRecord,
                        _,
                        DecompressAccountsIdempotent<'info>,
                        SeedParams,
                    >(
                        self.rent_sponsor.as_ref(),
                        cpi_accounts,
                        address_space,
                        &solana_accounts[i],
                        i,
                        &packed,
                        &meta,
                        post_system_accounts,
                        &mut compressed_pda_infos,
                        &crate::ID,
                        self,
                        None,
                    )?;
                }
                CompressedAccountVariant::PackedCTokenData(mut data) => {
                    data.token_data.version = 3;
                    compressed_token_accounts.push((data, meta));
                }
                CompressedAccountVariant::UserRecord(_)
                | CompressedAccountVariant::GameSession(_)
                | CompressedAccountVariant::PlaceholderRecord(_)
                | CompressedAccountVariant::CTokenData(_) => {
                    unreachable!("Unpacked variants should not appear during decompression")
                }
            }
        }

        Ok((compressed_pda_infos, compressed_token_accounts))
    }

    fn process_tokens<'b>(
        &self,
        _remaining_accounts: &[AccountInfo<'info>],
        _fee_payer: &AccountInfo<'info>,
        _ctoken_program: &AccountInfo<'info>,
        _ctoken_rent_sponsor: &AccountInfo<'info>,
        _ctoken_cpi_authority: &AccountInfo<'info>,
        _ctoken_config: &AccountInfo<'info>,
        _config: &AccountInfo<'info>,
        ctoken_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
        proof: light_sdk::instruction::ValidityProof,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        post_system_accounts: &[AccountInfo<'info>],
        has_pdas: bool,
    ) -> std::result::Result<(), ProgramError> {
        if ctoken_accounts.is_empty() {
            return Ok(());
        }

        light_compressed_token_sdk::compressible::decompress_runtime::process_decompress_tokens_runtime::<
            CTokenAccountVariant,
            _,
        >(
            self,
            _remaining_accounts,
            _fee_payer,
            _ctoken_program,
            _ctoken_rent_sponsor,
            _ctoken_cpi_authority,
            _ctoken_config,
            _config,
            ctoken_accounts,
            proof,
            cpi_accounts,
            post_system_accounts,
            has_pdas,
            &crate::ID,
        )?;

        Ok(())
    }
}

#[inline(never)]
pub fn process_decompress_accounts_idempotent<'info>(
    accounts: &DecompressAccountsIdempotent<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<CompressedAccountData>,
    proof: ValidityProof,
    system_accounts_offset: u8,
) -> Result<()> {
    light_sdk::compressible::process_decompress_accounts_idempotent(
        accounts,
        remaining_accounts,
        compressed_accounts,
        proof,
        system_accounts_offset,
        LIGHT_CPI_SIGNER,
        &crate::ID,
        None, // No seed params needed for manual implementation
    )
    .map_err(|e| e.into())
}

impl<'info> light_sdk::compressible::CompressContext<'info> for CompressAccountsIdempotent<'info> {
    fn fee_payer(&self) -> &AccountInfo<'info> {
        self.fee_payer.as_ref()
    }

    fn config(&self) -> &AccountInfo<'info> {
        &self.config
    }

    fn rent_sponsor(&self) -> &AccountInfo<'info> {
        &self.rent_sponsor
    }

    fn compression_authority(&self) -> &AccountInfo<'info> {
        &self.compression_authority
    }

    fn compress_pda_account(
        &self,
        account_info: &AccountInfo<'info>,
        meta: &CompressedAccountMetaNoLamportsNoAddress,
        cpi_accounts: &CpiAccounts<'_, 'info>,
        compression_config: &CompressibleConfig,
        program_id: &Pubkey,
    ) -> std::result::Result<Option<CompressedAccountInfo>, ProgramError> {
        let data = account_info.try_borrow_data()?;
        let discriminator = &data[0..8];

        match discriminator {
            d if d == UserRecord::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data()?;
                let mut account_data = UserRecord::try_deserialize(&mut &data_borrow[..])?;
                drop(data_borrow);

                let compressed_info = prepare_account_for_compression::<UserRecord>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.address_space,
                )?;
                Ok(Some(compressed_info))
            }
            d if d == GameSession::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data()?;
                let mut account_data = GameSession::try_deserialize(&mut &data_borrow[..])?;
                drop(data_borrow);

                let compressed_info = prepare_account_for_compression::<GameSession>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.address_space,
                )?;
                Ok(Some(compressed_info))
            }
            d if d == PlaceholderRecord::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data()?;
                let mut account_data = PlaceholderRecord::try_deserialize(&mut &data_borrow[..])?;
                drop(data_borrow);

                let compressed_info = prepare_account_for_compression::<PlaceholderRecord>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.address_space,
                )?;
                Ok(Some(compressed_info))
            }
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[inline(never)]
pub fn process_compress_accounts_idempotent<'info>(
    accounts: &CompressAccountsIdempotent<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    system_accounts_offset: u8,
) -> Result<()> {
    light_sdk::compressible::process_compress_pda_accounts_idempotent(
        accounts,
        remaining_accounts,
        compressed_accounts,
        system_accounts_offset,
        LIGHT_CPI_SIGNER,
        &crate::ID,
    )
    .map_err(|e| e.into())
}
