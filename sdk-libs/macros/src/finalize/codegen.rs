//! Code generation for LightFinalize and LightPreInit trait implementations.
//!
//! Two-phase design:
//! - LightPreInit: Creates mints at START via CPI context write
//! - LightFinalize: Compresses PDAs at END and executes with proof
//!
//! This allows mints to be used during instruction body (for vault creation, minting, etc.)

use super::parse::{CompressibleField, LightMintField, ParsedCompressibleStruct};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate both trait implementations
pub fn generate_finalize_impl(parsed: &ParsedCompressibleStruct) -> TokenStream {
    let struct_name = &parsed.struct_name;
    let (impl_generics, ty_generics, where_clause) = parsed.generics.split_for_impl();

    // Get the params type from instruction args (first arg)
    let params_type = parsed
        .instruction_args
        .as_ref()
        .and_then(|args| args.first())
        .map(|arg| &arg.ty);

    let params_type = match params_type {
        Some(ty) => ty,
        None => {
            // No instruction args - generate no-op impls
            return quote! {
                #[automatically_derived]
                impl #impl_generics light_sdk::compressible::LightPreInit<'info, ()> for #struct_name #ty_generics #where_clause {
                    fn light_pre_init(
                        &mut self,
                        _remaining: &[solana_account_info::AccountInfo<'info>],
                        _params: &(),
                    ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
                        Ok(false)
                    }
                }

                #[automatically_derived]
                impl #impl_generics light_sdk::compressible::LightFinalize<'info, ()> for #struct_name #ty_generics #where_clause {
                    fn light_finalize(
                        &mut self,
                        _remaining: &[solana_account_info::AccountInfo<'info>],
                        _params: &(),
                        _has_pre_init: bool,
                    ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                        Ok(())
                    }
                }
            };
        }
    };

    let params_ident = parsed
        .instruction_args
        .as_ref()
        .and_then(|args| args.first())
        .map(|arg| &arg.name)
        .expect("params ident must exist if type exists");

    let has_pdas = !parsed.compressible_fields.is_empty();
    let has_mints = !parsed.light_mint_fields.is_empty();

    // Get fee payer field
    let fee_payer = parsed
        .fee_payer_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { fee_payer });

    let compression_config = parsed
        .compression_config_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { compression_config });

    // Generate LightPreInit impl
    let pre_init_body = if has_mints {
        generate_pre_init_mints(parsed, params_ident, &fee_payer)
    } else {
        quote! { Ok(false) }
    };

    // Generate LightFinalize impl
    let finalize_body = if has_pdas {
        generate_finalize_pdas(parsed, params_ident, &fee_payer, &compression_config, has_mints)
    } else if has_mints {
        // Mints only, no PDAs - execute the mints written in pre_init
        generate_finalize_mints_only(parsed, params_ident, &fee_payer)
    } else {
        quote! { Ok(()) }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics light_sdk::compressible::LightPreInit<'info, #params_type> for #struct_name #ty_generics #where_clause {
            fn light_pre_init(
                &mut self,
                _remaining: &[solana_account_info::AccountInfo<'info>],
                #params_ident: &#params_type,
            ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
                use anchor_lang::ToAccountInfo;
                #pre_init_body
            }
        }

        #[automatically_derived]
        impl #impl_generics light_sdk::compressible::LightFinalize<'info, #params_type> for #struct_name #ty_generics #where_clause {
            fn light_finalize(
                &mut self,
                _remaining: &[solana_account_info::AccountInfo<'info>],
                #params_ident: &#params_type,
                _has_pre_init: bool,
            ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                use anchor_lang::ToAccountInfo;
                #finalize_body
            }
        }
    }
}

/// Generate LightPreInit body that writes mints to CPI context
fn generate_pre_init_mints(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
) -> TokenStream {
    let mint_count = parsed.light_mint_fields.len();
    
    // All mints write to CPI context (first uses first_set_context, rest use set_context)
    let mint_writes: Vec<TokenStream> = parsed.light_mint_fields.iter().enumerate().map(|(idx, mint)| {
        let is_first = idx == 0;
        generate_mint_cpi_write(mint, params_ident, fee_payer, is_first)
    }).collect();

    quote! {
        // Build CPI accounts WITH CPI context for batching
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
            &self.#fee_payer,
            _remaining,
            light_sdk_types::cpi_accounts::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
        );

        // Build SystemAccountInfos from CpiAccounts
        let system_accounts = light_ctoken_sdk::ctoken::SystemAccountInfos {
            light_system_program: cpi_accounts.get_account_info(0)?.clone(),
            cpi_authority_pda: cpi_accounts.authority()?.clone(),
            registered_program_pda: cpi_accounts.registered_program_pda()?.clone(),
            account_compression_authority: cpi_accounts.account_compression_authority()?.clone(),
            account_compression_program: cpi_accounts.account_compression_program()?.clone(),
            system_program: cpi_accounts.system_program()?.clone(),
        };

        let cpi_context_account = cpi_accounts.cpi_context()?.clone();

        // Write all mints to CPI context
        #(#mint_writes)*

        Ok(true) // Signal that CPI context was used
    }
}

/// Generate a single mint CPI context write
fn generate_mint_cpi_write(
    mint: &LightMintField,
    _params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    is_first: bool,
) -> TokenStream {
    let mint_signer = &mint.mint_signer;
    let authority = &mint.authority;
    let decimals = &mint.decimals;
    let address_tree_info = &mint.address_tree_info;

    let first_set_context = is_first;
    let set_context = !is_first;

    // Use explicit signer_seeds if provided, otherwise empty (for non-PDA signers)
    let signer_seeds_tokens = if let Some(seeds) = &mint.signer_seeds {
        quote! { &[#seeds] }
    } else {
        quote! { &[] }
    };

    quote! {
        {
            let __tree_info = &#address_tree_info;
            let __tree_account = cpi_accounts.get_tree_account_info(__tree_info.address_merkle_tree_pubkey_index as usize)?;
            let __tree_pubkey: solana_pubkey::Pubkey = light_sdk::light_account_checks::AccountInfoTrait::pubkey(__tree_account);
            let mint_signer_key = self.#mint_signer.to_account_info().key;
            let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
                mint_signer_key,
                &__tree_pubkey,
            );
            let (mint_pda, _) = light_ctoken_sdk::ctoken::find_cmint_address(mint_signer_key);

            let cpi_ctx = light_ctoken_interface::instructions::mint_action::CpiContext {
                first_set_context: #first_set_context,
                set_context: #set_context,
                in_tree_index: __tree_info.address_merkle_tree_pubkey_index,
                in_queue_index: __tree_info.address_queue_pubkey_index,
                out_queue_index: __tree_info.address_queue_pubkey_index,
                token_out_queue_index: 0,
                assigned_account_index: 0,
                read_only_address_trees: [0; 4],
                address_tree_pubkey: __tree_pubkey.to_bytes(),
            };

            let write_params = light_ctoken_sdk::ctoken::CreateCMintCpiWriteParams::new(
                #decimals,
                __tree_info.root_index,
                *self.#authority.to_account_info().key,
                compression_address,
                mint_pda,
                cpi_ctx,
            );

            light_ctoken_sdk::ctoken::CreateCompressedMintCpiWriteCpi {
                mint_signer: self.#mint_signer.to_account_info(),
                authority: self.#authority.to_account_info(),
                payer: self.#fee_payer.to_account_info(),
                cpi_context_account: cpi_context_account.clone(),
                system_accounts: light_ctoken_sdk::ctoken::SystemAccountInfos {
                    light_system_program: system_accounts.light_system_program.clone(),
                    cpi_authority_pda: system_accounts.cpi_authority_pda.clone(),
                    registered_program_pda: system_accounts.registered_program_pda.clone(),
                    account_compression_authority: system_accounts.account_compression_authority.clone(),
                    account_compression_program: system_accounts.account_compression_program.clone(),
                    system_program: system_accounts.system_program.clone(),
                },
                params: write_params,
            }.invoke_signed(#signer_seeds_tokens)?;
        }
    }
}

/// Generate LightFinalize body for PDAs (with optional CPI context from pre_init)
fn generate_finalize_pdas(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
    has_mints: bool,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) =
        generate_pda_compress_blocks(&parsed.compressible_fields, params_ident);
    let compressible_count = parsed.compressible_fields.len() as u8;

    if has_mints {
        // PDAs + mints: Write PDAs to CPI context, execute with proof
        quote! {
            // Build CPI accounts WITH CPI context (mints already written in pre_init)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                light_sdk_types::cpi_accounts::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            // Load compression config
            let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            // Collect compressed infos for all compressible accounts
            let mut all_compressed_infos = Vec::with_capacity(#compressible_count as usize);
            #(#compress_blocks)*

            // Write PDAs to CPI context (mints already written), then execute
            use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #params_ident.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .invoke_execute_cpi_context(cpi_accounts)?;

            Ok(())
        }
    } else {
        // PDAs only: Direct invoke (no CPI context needed)
        quote! {
            // Build CPI accounts (no CPI context needed for PDAs-only)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );

            // Load compression config
            let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            // Collect compressed infos for all compressible accounts
            let mut all_compressed_infos = Vec::with_capacity(#compressible_count as usize);
            #(#compress_blocks)*

            // Execute Light System Program CPI directly with proof
            use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #params_ident.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .invoke(cpi_accounts)?;

            Ok(())
        }
    }
}

/// Generate LightFinalize body for mints-only (no PDAs)
/// Executes the mints that were written to CPI context in pre_init
fn generate_finalize_mints_only(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
) -> TokenStream {
    // Use the last mint to execute with CPI context
    let last_mint = &parsed.light_mint_fields[parsed.light_mint_fields.len() - 1];
    let mint_signer = &last_mint.mint_signer;
    let authority = &last_mint.authority;
    let decimals = &last_mint.decimals;
    let address_tree_info = &last_mint.address_tree_info;

    // Use explicit signer_seeds if provided, otherwise empty
    let signer_seeds_tokens = if let Some(seeds) = &last_mint.signer_seeds {
        quote! { &[#seeds] }
    } else {
        quote! { &[] }
    };

    quote! {
        // Build CPI accounts WITH CPI context (mints already written in pre_init)
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
            &self.#fee_payer,
            _remaining,
            light_sdk_types::cpi_accounts::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
        );

        let system_accounts = light_ctoken_sdk::ctoken::SystemAccountInfos {
            light_system_program: cpi_accounts.get_account_info(0)?.clone(),
            cpi_authority_pda: cpi_accounts.authority()?.clone(),
            registered_program_pda: cpi_accounts.registered_program_pda()?.clone(),
            account_compression_authority: cpi_accounts.account_compression_authority()?.clone(),
            account_compression_program: cpi_accounts.account_compression_program()?.clone(),
            system_program: cpi_accounts.system_program()?.clone(),
        };

        let cpi_context_account = cpi_accounts.cpi_context()?.clone();

        // Execute with the last mint
        {
            let __tree_info = &#address_tree_info;
            let address_tree = cpi_accounts.get_tree_account_info(__tree_info.address_merkle_tree_pubkey_index as usize)?;
            let output_queue = cpi_accounts.get_tree_account_info(__tree_info.address_queue_pubkey_index as usize)?;
            let __tree_pubkey: solana_pubkey::Pubkey = light_sdk::light_account_checks::AccountInfoTrait::pubkey(address_tree);

            let mint_signer_key = self.#mint_signer.to_account_info().key;
            let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
                mint_signer_key,
                &__tree_pubkey,
            );
            let (mint_pda, _) = light_ctoken_sdk::ctoken::find_cmint_address(mint_signer_key);

            let __proof: light_ctoken_sdk::CompressedProof = #params_ident.proof.0.clone()
                .expect("proof is required for mint creation");

            let __mint_params = light_ctoken_sdk::ctoken::CreateCMintParams {
                decimals: #decimals,
                address_merkle_tree_root_index: __tree_info.root_index,
                mint_authority: *self.#authority.to_account_info().key,
                proof: __proof,
                compression_address,
                mint: mint_pda,
                freeze_authority: None,
                extensions: None,
            };

            // Execute with CPI context
            light_ctoken_sdk::ctoken::CreateCMintCpi {
                mint_seed: self.#mint_signer.to_account_info(),
                authority: self.#authority.to_account_info(),
                payer: self.#fee_payer.to_account_info(),
                address_tree: address_tree.clone(),
                output_queue: output_queue.clone(),
                system_accounts,
                cpi_context: None,
                cpi_context_account: Some(cpi_context_account),
                params: __mint_params,
            }.invoke_signed(#signer_seeds_tokens)?;
        }

        Ok(())
    }
}

/// Generate compression blocks for PDA fields
fn generate_pda_compress_blocks(
    fields: &[CompressibleField],
    _params_ident: &syn::Ident,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut blocks = Vec::new();
    let mut addr_idents = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let idx_lit = idx as u8;
        let ident = &field.ident;
        let addr_tree_info = &field.address_tree_info;
        let output_tree = &field.output_tree;
        let acc_ty_path = extract_inner_account_type(&field.ty);

        let new_addr_params_ident = format_ident!("__new_addr_params_{}", idx);
        let compressed_infos_ident = format_ident!("__compressed_infos_{}", idx);
        let address_ident = format_ident!("__address_{}", idx);
        let account_info_ident = format_ident!("__account_info_{}", idx);
        let account_key_ident = format_ident!("__account_key_{}", idx);
        let account_data_ident = format_ident!("__account_data_{}", idx);

        addr_idents.push(quote! { #new_addr_params_ident });

        blocks.push(quote! {
            // Get account info early before any mutable borrows
            let #account_info_ident = self.#ident.to_account_info();
            let #account_key_ident = #account_info_ident.key.to_bytes();

            let #new_addr_params_ident = {
                let tree_info = &#addr_tree_info;
                let __seed: [u8; 32] = light_sdk::address::v1::derive_address_seed(
                    &[
                        #account_key_ident.as_ref(),
                    ],
                    &crate::ID,
                ).into();
                light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked {
                    seed: __seed,
                    address_merkle_tree_account_index: tree_info.address_merkle_tree_pubkey_index,
                    address_queue_account_index: tree_info.address_queue_pubkey_index,
                    address_merkle_tree_root_index: tree_info.root_index,
                    assigned_to_account: true,
                    assigned_account_index: #idx_lit,
                }
            };

            // Derive the compressed address
            let #address_ident = light_compressed_account::address::derive_address(
                &#account_key_ident,
                &cpi_accounts
                    .get_tree_account_info(#new_addr_params_ident.address_merkle_tree_account_index as usize)?
                    .key()
                    .to_bytes(),
                &crate::ID.to_bytes(),
            );

            // Get mutable reference to inner account data
            let #account_data_ident = &mut **self.#ident;

            let #compressed_infos_ident = light_sdk::compressible::prepare_compressed_account_on_init::<#acc_ty_path>(
                &#account_info_ident,
                #account_data_ident,
                &compression_config_data,
                #address_ident,
                #new_addr_params_ident,
                #output_tree,
                &cpi_accounts,
                &compression_config_data.address_space,
                false,
            )?;
            all_compressed_infos.push(#compressed_infos_ident);
        });
    }

    (blocks, addr_idents)
}

/// Extract the inner type T from Account<'info, T> or Box<Account<'info, T>>
fn extract_inner_account_type(ty: &syn::Type) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;
            if let Some(segment) = path.segments.last() {
                let ident_str = segment.ident.to_string();

                if ident_str == "Account" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                return quote! { #inner_ty };
                            }
                        }
                    }
                }

                if ident_str == "Box" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return extract_inner_account_type(inner);
                        }
                    }
                }
            }
            quote! { #ty }
        }
        _ => quote! { #ty },
    }
}

