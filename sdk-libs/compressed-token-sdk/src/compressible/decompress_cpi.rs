#![cfg(feature = "anchor")]
use std::marker::PhantomData;

use crate::{
    account2::CTokenAccount2,
    compressible::{create_compressible_token_account, PackedCompressedTokenDataWithContext},
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    CompressedCpiContext,
};
use anchor_lang::{
    prelude::{AccountInfo, AccountMeta},
    solana_program::program::invoke_signed,
    Key,
};
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::{
    instructions::transfer2::{Compression, MultiTokenTransferOutputData},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_sdk::{cpi::CpiAccountsSmall, error::LightSdkError, instruction::ValidityProof};

pub trait TokenVariant<'c, 'info> {
    fn get_seeds(
        &self,
        input: &DecompressAccountsInput<'c, 'info>,
        token: &PackedCompressedTokenDataWithContext,
    ) -> Result<Vec<Vec<u8>>, LightSdkError>;
}

#[derive(Debug, Clone)]
pub struct DecompressTokenAccount<'c, 'info, V: TokenVariant<'c, 'info>> {
    pub variant: V, // 1 byte
    pub token_data: PackedCompressedTokenDataWithContext,
    pub phantom_data: PhantomData<(&'c (), &'info ())>,
}

pub struct DecompressAccountsInput<'c, 'info> {
    pub fee_payer: &'c AccountInfo<'info>,
    pub rent_payer: &'c AccountInfo<'info>,
    pub config: &'c AccountInfo<'info>,
    pub compressed_token_program: &'c AccountInfo<'info>,
    pub compressed_token_cpi_authority: &'c AccountInfo<'info>,
    pub remaining_accounts: &'c [AccountInfo<'info>],
    pub cpi_accounts: &'c CpiAccountsSmall<'c, 'info>,
    pub proof: ValidityProof,
    pub has_tokens: bool,
    pub has_pdas: bool,
}

pub fn process_compressed_token_accounts<'c, 'info, V: TokenVariant<'c, 'info>>(
    compressed_token_accounts: Vec<DecompressTokenAccount<'c, 'info, V>>,
    inputs: DecompressAccountsInput<'c, 'info>,
) -> Result<(), TokenSdkError> {
    let DecompressAccountsInput {
        fee_payer,
        rent_payer,
        config,
        remaining_accounts,
        cpi_accounts,
        proof,
        has_tokens,
        has_pdas,
        ..
    } = inputs;
    let mut compressed_token_infos = Vec::with_capacity(compressed_token_accounts.len());
    let mut all_compressed_token_signers_seeds = Vec::with_capacity(20);

    // creates account_metas for CPI.
    let tree_accounts = ProgramPackedAccounts {
        accounts: cpi_accounts.tree_accounts()?,
    };
    let mut packed_accounts = Vec::with_capacity(tree_accounts.accounts.len());
    for account_info in tree_accounts.accounts {
        packed_accounts.push(account_meta_from_account_info(account_info));
    }

    // step 2: decompressing the token accounts + settle cpi
    for compressed_token_account in compressed_token_accounts.into_iter() {
        let owner_index = compressed_token_account
            .token_data
            .multi_input_token_data_with_context
            .owner;
        let mint_index = compressed_token_account.token_data.mint;
        let system_program = cpi_accounts.system_program()?;

        let token_account = &tree_accounts.get_u8(owner_index, "owner")?;

        let mint_info = &tree_accounts.get_u8(mint_index, "mint")?;

        // seeds for ctoken. match on variant.
        let ctoken_signer_seeds = compressed_token_account
            .variant
            .get_seeds(&inputs, &compressed_token_account.token_data)?;

        let in_token_data = compressed_token_account
            .token_data
            .multi_input_token_data_with_context
            .clone();
        let amount = in_token_data.amount;
        let mint = compressed_token_account.token_data.mint;
        let source_or_recipient = compressed_token_account
            .token_data
            .source_or_recipient_token_account;

        let compression = Compression::decompress_ctoken(amount, mint, source_or_recipient);

        let ctoken_account = CTokenAccount2 {
            inputs: vec![in_token_data],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(compression),
            delegate_is_set: false,
            method_used: true,
        };
        create_compressible_token_account(
            token_account,
            &fee_payer.clone(),
            token_account,
            mint_info,
            &system_program.clone(),
            &inputs.compressed_token_program.clone(),
            &ctoken_signer_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>(),
            &fee_payer,
            &fee_payer,
            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as u64,
        )?;
        packed_accounts[owner_index as usize].is_signer = true;

        compressed_token_infos.push(ctoken_account);
        all_compressed_token_signers_seeds.extend(ctoken_signer_seeds);
    }

    if has_tokens && has_pdas {
        let cpi_context = cpi_accounts
            .cpi_context()
            .inspect_err(|_| {
                solana_msg::msg!(
                    "cpi context account is None but decompression has tokens and pdas."
                );
            })?
            .key();
        // CPI with CPI_CONTEXT
        let cpi_inputs = Transfer2Inputs {
            validity_proof: proof,
            transfer_config: Transfer2Config::new()
                .with_cpi_context(
                    cpi_context,
                    CompressedCpiContext {
                        set_context: false,           // settlement.
                        first_set_context: false,     // settlement.
                        cpi_context_account_index: 0, // We expect the cpi context to be in index 0.
                    },
                )
                .filter_zero_amount_outputs(),
            meta_config: Transfer2AccountsMetaConfig::new_with_cpi_context(
                fee_payer.key(),
                packed_accounts,
                cpi_context,
            ),
            in_lamports: None,
            out_lamports: None,
            token_accounts: compressed_token_infos,
        };

        let ctoken_ix = create_transfer2_instruction(cpi_inputs)?;

        // account_infos
        let mut all_account_infos = vec![fee_payer.clone()];
        all_account_infos.extend_from_slice(remaining_accounts);
        all_account_infos.push(inputs.compressed_token_cpi_authority.clone());

        // ctoken cpi
        let seed_refs = all_compressed_token_signers_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>();

        invoke_signed(
            &ctoken_ix,
            all_account_infos.as_slice(),
            &[seed_refs.as_slice()],
        )?;
    } else if has_tokens {
        // CPI without CPI_CONTEXT
        let transfer_inputs = Transfer2Inputs {
            validity_proof: proof,
            transfer_config: Transfer2Config::new().filter_zero_amount_outputs(),
            meta_config: Transfer2AccountsMetaConfig::new(fee_payer.key(), packed_accounts),
            in_lamports: None,
            out_lamports: None,
            token_accounts: compressed_token_infos,
        };

        let ctoken_ix = create_transfer2_instruction(transfer_inputs).unwrap();

        // account_infos
        let mut all_account_infos = Vec::with_capacity(5 + cpi_accounts.to_account_infos().len());
        all_account_infos.push(fee_payer.clone());
        all_account_infos.push(inputs.compressed_token_cpi_authority.clone());
        all_account_infos.push(inputs.compressed_token_program.clone());
        all_account_infos.push(rent_payer.clone());
        all_account_infos.push(config.clone());
        all_account_infos.extend(cpi_accounts.to_account_infos());

        // ctoken cpi
        let seed_refs = all_compressed_token_signers_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>();
        invoke_signed(
            &ctoken_ix,
            all_account_infos.as_slice(),
            &[seed_refs.as_slice()],
        )?;
    } else {
        anchor_lang::prelude::msg!("no token accounts provided");
        panic!()
    }

    Ok(())
}

#[inline]
pub fn account_meta_from_account_info(account_info: &AccountInfo) -> AccountMeta {
    AccountMeta {
        pubkey: *account_info.key,
        is_signer: account_info.is_signer,
        is_writable: account_info.is_writable,
    }
}
