//! Runtime for compress_accounts_idempotent instruction.
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_sdk_types::{
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub trait CompressContext<'info> {
    fn fee_payer(&self) -> &AccountInfo<'info>;
    fn config(&self) -> &AccountInfo<'info>;
    fn rent_sponsor(&self) -> &AccountInfo<'info>;
    fn compression_authority(&self) -> &AccountInfo<'info>;

    fn compress_pda_account(
        &self,
        account_info: &AccountInfo<'info>,
        meta: &CompressedAccountMetaNoLamportsNoAddress,
        cpi_accounts: &crate::cpi::v2::CpiAccounts<'_, 'info>,
        compression_config: &crate::compressible::CompressibleConfig,
        program_id: &Pubkey,
    ) -> Result<Option<CompressedAccountInfo>, ProgramError>;
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_compress_pda_accounts_idempotent<'info, Ctx>(
    ctx: &Ctx,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    system_accounts_offset: u8,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    Ctx: CompressContext<'info>,
{
    use crate::cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    };

    let proof = crate::instruction::ValidityProof::new(None);

    let compression_config =
        crate::compressible::CompressibleConfig::load_checked(ctx.config(), program_id)?;

    if *ctx.rent_sponsor().key != compression_config.rent_sponsor {
        return Err(ProgramError::Custom(0));
    }

    let cpi_accounts = CpiAccounts::new(
        ctx.fee_payer(),
        &remaining_accounts[system_accounts_offset as usize..],
        cpi_signer,
    );

    let mut compressed_pda_infos: Vec<CompressedAccountInfo> =
        Vec::with_capacity(compressed_accounts.len());
    let mut pda_indices_to_close: Vec<usize> = Vec::with_capacity(compressed_accounts.len());

    let system_accounts_start = cpi_accounts.system_accounts_end_offset();
    let all_post_system = &cpi_accounts.to_account_infos()[system_accounts_start..];

    // PDAs are at the end of remaining_accounts, after all the merkle tree/queue accounts
    let pda_start_in_all_accounts = all_post_system.len() - compressed_accounts.len();
    let solana_accounts = &all_post_system[pda_start_in_all_accounts..];

    for (i, account_info) in solana_accounts.iter().enumerate() {
        if account_info.data_is_empty() {
            continue;
        }

        if account_info.owner != program_id {
            continue;
        }

        let meta = compressed_accounts[i];

        if let Some(compressed_info) = ctx.compress_pda_account(
            account_info,
            &meta,
            &cpi_accounts,
            &compression_config,
            program_id,
        )? {
            compressed_pda_infos.push(compressed_info);
            pda_indices_to_close.push(i);
        }
    }

    if !compressed_pda_infos.is_empty() {
        LightSystemProgramCpi::new_cpi(cpi_signer, proof)
            .with_account_infos(&compressed_pda_infos)
            .invoke(cpi_accounts.clone())?;

        for idx in pda_indices_to_close {
            let mut info = solana_accounts[idx].clone();
            crate::compressible::close::close(&mut info, ctx.rent_sponsor().clone())
                .map_err(ProgramError::from)?;
        }
    }

    Ok(())
}
