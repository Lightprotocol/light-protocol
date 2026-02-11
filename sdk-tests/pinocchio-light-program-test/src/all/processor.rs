use light_account_pinocchio::{
    prepare_compressed_account_on_init, CompressedCpiContext, CpiAccounts, CpiAccountsConfig,
    CpiContextWriteAccounts, CreateMints, CreateMintsStaticAccounts, CreateTokenAccountCpi,
    CreateTokenAtaCpi, InstructionDataInvokeCpiWithAccountInfo, InvokeLightSystemProgram,
    LightAccount, LightConfig, LightSdkTypesError, PackedAddressTreeInfoExt, SingleMintParams,
};
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};

use super::accounts::{CreateAllAccounts, CreateAllParams};

pub fn process(
    ctx: &CreateAllAccounts<'_>,
    params: &CreateAllParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    use borsh::BorshDeserialize;

    const NUM_LIGHT_PDAS: usize = 2;
    const NUM_LIGHT_MINTS: usize = 1;
    const WITH_CPI_CONTEXT: bool = true;

    // 1. Build CPI accounts
    let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
    if remaining_accounts.len() < system_accounts_offset {
        return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
    }
    let config = CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        ctx.payer,
        &remaining_accounts[system_accounts_offset..],
        config,
    );

    // 2. Address tree info
    let address_tree_info = &params.create_accounts_proof.address_tree_info;
    let address_tree_pubkey = address_tree_info
        .get_tree_pubkey(&cpi_accounts)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let output_tree_index = params.create_accounts_proof.output_state_tree_index;

    // 3. Load config, get slot
    let light_config = LightConfig::load_checked(ctx.compression_config, &crate::ID)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let current_slot = Clock::get()
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
        .slot;

    // 4. Create PDAs via invoke_write_to_cpi_context_first
    {
        let cpi_context = CompressedCpiContext::first();
        let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
        let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

        // 4a. Borsh PDA (index 0)
        let borsh_record_key = *ctx.borsh_record.key();
        prepare_compressed_account_on_init(
            &borsh_record_key,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            0,
            &crate::ID,
            &mut new_address_params,
            &mut account_infos,
        )?;
        {
            let mut account_data = ctx
                .borsh_record
                .try_borrow_mut_data()
                .map_err(|_| LightSdkTypesError::Borsh)?;
            let mut record = crate::state::MinimalRecord::try_from_slice(&account_data[8..])
                .map_err(|_| LightSdkTypesError::Borsh)?;
            record.set_decompressed(&light_config, current_slot);
            let serialized = borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
            account_data[8..8 + serialized.len()].copy_from_slice(&serialized);
        }

        // 4b. ZeroCopy PDA (index 1)
        let zero_copy_record_key = *ctx.zero_copy_record.key();
        prepare_compressed_account_on_init(
            &zero_copy_record_key,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            1,
            &crate::ID,
            &mut new_address_params,
            &mut account_infos,
        )?;
        {
            let mut account_data = ctx
                .zero_copy_record
                .try_borrow_mut_data()
                .map_err(|_| LightSdkTypesError::Borsh)?;
            let record_bytes =
                &mut account_data[8..8 + core::mem::size_of::<crate::state::ZeroCopyRecord>()];
            let record: &mut crate::state::ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
            record.set_decompressed(&light_config, current_slot);
        }

        // 4c. Write to CPI context
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

        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority()?,
            cpi_context: cpi_accounts.cpi_context()?,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
        };
        instruction_data.invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    }

    // 5. Create Mint
    {
        let authority_key = *ctx.authority.key();
        let mint_signer_key = *ctx.mint_signer.key();

        let mint_signer_seeds: &[&[u8]] = &[
            crate::MINT_SIGNER_SEED_A,
            authority_key.as_ref(),
            &[params.mint_signer_bump],
        ];

        let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [SingleMintParams {
            decimals: 9,
            mint_authority: authority_key,
            mint_bump: None,
            freeze_authority: None,
            mint_seed_pubkey: mint_signer_key,
            authority_seeds: None,
            mint_signer_seeds: Some(mint_signer_seeds),
            token_metadata: None,
        }];

        CreateMints {
            mints: &sdk_mints,
            proof_data: &params.create_accounts_proof,
            mint_seed_accounts: ctx.mint_signers_slice,
            mint_accounts: ctx.mints_slice,
            static_accounts: CreateMintsStaticAccounts {
                fee_payer: ctx.payer,
                compressible_config: ctx.compressible_config,
                rent_sponsor: ctx.rent_sponsor,
                cpi_authority: ctx.cpi_authority,
            },
            cpi_context_offset: NUM_LIGHT_PDAS as u8,
        }
        .invoke(&cpi_accounts)?;
    }

    // 6. Create Token Vault
    {
        let mint_key = *ctx.mint.key();
        let vault_seeds: &[&[u8]] = &[
            crate::VAULT_SEED,
            mint_key.as_ref(),
            &[params.token_vault_bump],
        ];

        CreateTokenAccountCpi {
            payer: ctx.payer,
            account: ctx.token_vault,
            mint: ctx.mint,
            owner: *ctx.vault_owner.key(),
        }
        .rent_free(
            ctx.compressible_config,
            ctx.rent_sponsor,
            ctx.system_program,
            &crate::ID,
        )
        .invoke_signed(vault_seeds)?;
    }

    // 7. Create ATA
    {
        CreateTokenAtaCpi {
            payer: ctx.payer,
            owner: ctx.ata_owner,
            mint: ctx.mint,
            ata: ctx.user_ata,
        }
        .rent_free(
            ctx.compressible_config,
            ctx.rent_sponsor,
            ctx.system_program,
        )
        .invoke()?;
    }

    Ok(())
}
