//! Code generation for LightFinalize and LightPreInit trait implementations.
//!
//! Design for mints:
//! - At mint init, we CREATE + DECOMPRESS atomically
//! - After init, the CMint should always be in decompressed/"hot" state
//!
//! Flow for PDAs + mints:
//! 1. Pre-init: ALL compression logic executes here
//!    a. Write PDAs to CPI context
//!    b. Invoke mint_action with decompress + CPI context
//!    c. CMint is now "hot" and usable
//! 2. Instruction body: Can use hot CMint (mintTo, transfers, etc.)
//! 3. Finalize: No-op (all work done in pre_init)

use super::parse::{ParsedCompressibleStruct, RentFreeField};
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

    let has_pdas = !parsed.rentfree_fields.is_empty();
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

    // CToken accounts for decompress
    let ctoken_config = parsed
        .ctoken_config_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { ctoken_compressible_config });

    let ctoken_rent_sponsor = parsed
        .ctoken_rent_sponsor_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { ctoken_rent_sponsor });

    let ctoken_program = parsed
        .ctoken_program_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { ctoken_program });

    let ctoken_cpi_authority = parsed
        .ctoken_cpi_authority_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { ctoken_cpi_authority });

    // Generate LightPreInit impl based on what we have
    // ALL compression logic runs in pre_init so instruction body can use hot state
    let pre_init_body = if has_pdas && has_mints {
        // PDAs + mints: Write PDAs to CPI context, then invoke mint_action with decompress
        generate_pre_init_pdas_and_mints(
            parsed,
            params_ident,
            &fee_payer,
            &compression_config,
            &ctoken_config,
            &ctoken_rent_sponsor,
            &ctoken_program,
            &ctoken_cpi_authority,
        )
    } else if has_mints {
        // Mints only: Invoke mint_action with decompress (no CPI context)
        generate_pre_init_mints_only(
            parsed,
            params_ident,
            &fee_payer,
            &ctoken_config,
            &ctoken_rent_sponsor,
            &ctoken_program,
            &ctoken_cpi_authority,
        )
    } else if has_pdas {
        // PDAs only: Direct invoke (no CPI context needed)
        generate_pre_init_pdas_only(parsed, params_ident, &fee_payer, &compression_config)
    } else {
        quote! { Ok(false) }
    };

    // LightFinalize: No-op (all work done in pre_init)
    let finalize_body = quote! { Ok(()) };

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

/// Generate LightPreInit body for PDAs + mints:
/// 1. Write PDAs to CPI context
/// 2. Invoke mint_action with decompress + CPI context
/// After this, CMint is "hot" and usable in instruction body
#[allow(clippy::too_many_arguments)]
fn generate_pre_init_pdas_and_mints(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
    ctoken_config: &TokenStream,
    ctoken_rent_sponsor: &TokenStream,
    ctoken_program: &TokenStream,
    ctoken_cpi_authority: &TokenStream,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) =
        generate_pda_compress_blocks(&parsed.rentfree_fields, params_ident);
    let rentfree_count = parsed.rentfree_fields.len() as u8;
    let pda_count = parsed.rentfree_fields.len();

    // Get the first PDA's output tree index (for the state tree output queue)
    let first_pda_output_tree = &parsed.rentfree_fields[0].output_tree;

    // Get the first mint (we only support one mint currently)
    let mint = &parsed.light_mint_fields[0];
    let mint_field_ident = &mint.field_ident;
    let mint_signer = &mint.mint_signer;
    let authority = &mint.authority;
    let decimals = &mint.decimals;
    let address_tree_info = &mint.address_tree_info;

    // Use explicit signer_seeds if provided, otherwise empty
    let signer_seeds_tokens = if let Some(seeds) = &mint.signer_seeds {
        quote! { #seeds }
    } else {
        quote! { &[] as &[&[u8]] }
    };

    // Build freeze_authority expression
    let freeze_authority_tokens = if let Some(freeze_auth) = &mint.freeze_authority {
        quote! { Some(*self.#freeze_auth.to_account_info().key) }
    } else {
        quote! { None }
    };

    // rent_payment defaults to 2 epochs (u8)
    let rent_payment_tokens = if let Some(rent) = &mint.rent_payment {
        quote! { #rent }
    } else {
        quote! { 2u8 }
    };

    // write_top_up defaults to 0 (u32)
    let write_top_up_tokens = if let Some(top_up) = &mint.write_top_up {
        quote! { #top_up }
    } else {
        quote! { 0u32 }
    };

    // assigned_account_index for mint is after PDAs
    let mint_assigned_index = pda_count as u8;

    quote! {
        // Build CPI accounts WITH CPI context for batching
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

        // Collect compressed infos for all rentfree PDA accounts
        let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
        #(#compress_blocks)*

        // Step 1: Write PDAs to CPI context
        let cpi_context_account = cpi_accounts.cpi_context()?;
        let cpi_context_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority()?,
            cpi_context: cpi_context_account,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
        };

        use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
        light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
            crate::LIGHT_CPI_SIGNER,
            #params_ident.create_accounts_proof.proof.clone()
        )
            .with_new_addresses(&[#(#new_addr_idents),*])
            .with_account_infos(&all_compressed_infos)
            .write_to_cpi_context_first()
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

        // Step 2: Build and invoke mint_action with decompress + CPI context
        {
            let __tree_info = &#address_tree_info;
            let address_tree = cpi_accounts.get_tree_account_info(__tree_info.address_merkle_tree_pubkey_index as usize)?;
            // Output queue is the state tree queue (same as the PDAs' output tree)
            let __output_tree_index = #first_pda_output_tree;
            let output_queue = cpi_accounts.get_tree_account_info(__output_tree_index as usize)?;
            let __tree_pubkey: solana_pubkey::Pubkey = light_sdk::light_account_checks::AccountInfoTrait::pubkey(address_tree);

            let mint_signer_key = self.#mint_signer.to_account_info().key;
            let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
                mint_signer_key,
                &__tree_pubkey,
            );
            let (mint_pda, cmint_bump) = light_ctoken_sdk::ctoken::find_cmint_address(mint_signer_key);

            let __proof: light_ctoken_sdk::CompressedProof = #params_ident.create_accounts_proof.proof.0.clone()
                .expect("proof is required for mint creation");

            let __freeze_authority: Option<solana_pubkey::Pubkey> = #freeze_authority_tokens;

            // Build compressed mint instruction data
            let compressed_mint_data = light_ctoken_interface::instructions::mint_action::CompressedMintInstructionData {
                supply: 0,
                decimals: #decimals,
                metadata: light_ctoken_interface::state::CompressedMintMetadata {
                    version: 3,
                    mint: mint_pda.to_bytes().into(),
                    cmint_decompressed: false,
                    compressed_address: compression_address,
                },
                mint_authority: Some((*self.#authority.to_account_info().key).to_bytes().into()),
                freeze_authority: __freeze_authority.map(|a| a.to_bytes().into()),
                extensions: None,
            };

            // Build mint action instruction data with decompress
            let mut instruction_data = light_ctoken_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                __tree_info.root_index,
                __proof,
                compressed_mint_data,
            )
            .with_decompress_mint(light_ctoken_interface::instructions::mint_action::DecompressMintAction {
                cmint_bump,
                rent_payment: #rent_payment_tokens,
                write_top_up: #write_top_up_tokens,
            })
            .with_cpi_context(light_ctoken_interface::instructions::mint_action::CpiContext {
                address_tree_pubkey: __tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false, // PDAs already wrote to context
                // in_tree_index is 1-indexed and points to the state queue (for CPI context validation)
                // The Light System Program does `in_tree_index - 1` and uses queue's associated_merkle_tree
                in_tree_index: __output_tree_index + 1, // +1 because 1-indexed
                in_queue_index: __output_tree_index,
                out_queue_index: __output_tree_index, // Output state queue
                token_out_queue_index: 0,
                assigned_account_index: #mint_assigned_index,
                read_only_address_trees: [0; 4],
            });

            // Build account metas with compressible CMint
            let mut meta_config = light_ctoken_sdk::compressed_token::mint_action::MintActionMetaConfig::new_create_mint(
                *self.#fee_payer.to_account_info().key,
                *self.#authority.to_account_info().key,
                *mint_signer_key,
                __tree_pubkey,
                *output_queue.key,
            )
            .with_compressible_cmint(
                mint_pda,
                *self.#ctoken_config.to_account_info().key,
                *self.#ctoken_rent_sponsor.to_account_info().key,
            );

            meta_config.cpi_context = Some(*cpi_accounts.cpi_context()?.key);

            let account_metas = meta_config.to_account_metas();

            use light_compressed_account::instruction_data::traits::LightInstructionData;
            let ix_data = instruction_data.data()
                .map_err(|e| light_sdk::error::LightSdkError::Borsh)?;

            let mint_action_ix = anchor_lang::solana_program::instruction::Instruction {
                program_id: solana_pubkey::Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID),
                accounts: account_metas,
                data: ix_data,
            };

            // Build account infos and invoke
            // Include all accounts needed for mint_action with decompress
            let mut account_infos = cpi_accounts.to_account_infos();
            // Add ctoken-specific accounts that aren't in the Light System CPI accounts
            account_infos.push(self.#ctoken_program.to_account_info());
            account_infos.push(self.#ctoken_cpi_authority.to_account_info());
            account_infos.push(self.#mint_field_ident.to_account_info());
            account_infos.push(self.#ctoken_config.to_account_info());
            account_infos.push(self.#ctoken_rent_sponsor.to_account_info());
            account_infos.push(self.#authority.to_account_info());
            account_infos.push(self.#mint_signer.to_account_info());
            account_infos.push(self.#fee_payer.to_account_info());

            let signer_seeds: &[&[u8]] = #signer_seeds_tokens;
            if signer_seeds.is_empty() {
                anchor_lang::solana_program::program::invoke(&mint_action_ix, &account_infos)?;
            } else {
                anchor_lang::solana_program::program::invoke_signed(&mint_action_ix, &account_infos, &[signer_seeds])?;
            }
        }

        Ok(true)
    }
}

/// Generate LightPreInit body for mints-only (no PDAs):
/// Invoke mint_action with decompress directly
/// After this, CMint is "hot" and usable in instruction body
#[allow(clippy::too_many_arguments)]
fn generate_pre_init_mints_only(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    ctoken_config: &TokenStream,
    ctoken_rent_sponsor: &TokenStream,
    ctoken_program: &TokenStream,
    ctoken_cpi_authority: &TokenStream,
) -> TokenStream {
    // Get the first mint (we only support one mint currently)
    let mint = &parsed.light_mint_fields[0];
    let mint_field_ident = &mint.field_ident;
    let mint_signer = &mint.mint_signer;
    let authority = &mint.authority;
    let decimals = &mint.decimals;
    let address_tree_info = &mint.address_tree_info;

    // Use explicit signer_seeds if provided, otherwise empty
    let signer_seeds_tokens = if let Some(seeds) = &mint.signer_seeds {
        quote! { #seeds }
    } else {
        quote! { &[] as &[&[u8]] }
    };

    // Build freeze_authority expression
    let freeze_authority_tokens = if let Some(freeze_auth) = &mint.freeze_authority {
        quote! { Some(*self.#freeze_auth.to_account_info().key) }
    } else {
        quote! { None }
    };

    // rent_payment defaults to 2 epochs (u8)
    let rent_payment_tokens = if let Some(rent) = &mint.rent_payment {
        quote! { #rent }
    } else {
        quote! { 2u8 }
    };

    // write_top_up defaults to 0 (u32)
    let write_top_up_tokens = if let Some(top_up) = &mint.write_top_up {
        quote! { #top_up }
    } else {
        quote! { 0u32 }
    };

    quote! {
        // Build CPI accounts (no CPI context needed for mints-only)
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
            &self.#fee_payer,
            _remaining,
            crate::LIGHT_CPI_SIGNER,
        );

        // Build and invoke mint_action with decompress
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
            let (mint_pda, cmint_bump) = light_ctoken_sdk::ctoken::find_cmint_address(mint_signer_key);

            let __proof: light_ctoken_sdk::CompressedProof = #params_ident.create_accounts_proof.proof.0.clone()
                .expect("proof is required for mint creation");

            let __freeze_authority: Option<solana_pubkey::Pubkey> = #freeze_authority_tokens;

            // Build compressed mint instruction data
            let compressed_mint_data = light_ctoken_interface::instructions::mint_action::CompressedMintInstructionData {
                supply: 0,
                decimals: #decimals,
                metadata: light_ctoken_interface::state::CompressedMintMetadata {
                    version: 3,
                    mint: mint_pda.to_bytes().into(),
                    cmint_decompressed: false,
                    compressed_address: compression_address,
                },
                mint_authority: Some((*self.#authority.to_account_info().key).to_bytes().into()),
                freeze_authority: __freeze_authority.map(|a| a.to_bytes().into()),
                extensions: None,
            };

            // Build mint action instruction data with decompress (no CPI context)
            let instruction_data = light_ctoken_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                __tree_info.root_index,
                __proof,
                compressed_mint_data,
            )
            .with_decompress_mint(light_ctoken_interface::instructions::mint_action::DecompressMintAction {
                cmint_bump,
                rent_payment: #rent_payment_tokens,
                write_top_up: #write_top_up_tokens,
            });

            // Build account metas with compressible CMint
            let meta_config = light_ctoken_sdk::compressed_token::mint_action::MintActionMetaConfig::new_create_mint(
                *self.#fee_payer.to_account_info().key,
                *self.#authority.to_account_info().key,
                *mint_signer_key,
                __tree_pubkey,
                *output_queue.key,
            )
            .with_compressible_cmint(
                mint_pda,
                *self.#ctoken_config.to_account_info().key,
                *self.#ctoken_rent_sponsor.to_account_info().key,
            );

            let account_metas = meta_config.to_account_metas();

            use light_compressed_account::instruction_data::traits::LightInstructionData;
            let ix_data = instruction_data.data()
                .map_err(|e| light_sdk::error::LightSdkError::Borsh)?;

            let mint_action_ix = anchor_lang::solana_program::instruction::Instruction {
                program_id: solana_pubkey::Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID),
                accounts: account_metas,
                data: ix_data,
            };

            // Build account infos and invoke
            let mut account_infos = cpi_accounts.to_account_infos();
            // Add ctoken-specific accounts
            account_infos.push(self.#ctoken_program.to_account_info());
            account_infos.push(self.#ctoken_cpi_authority.to_account_info());
            account_infos.push(self.#mint_field_ident.to_account_info());
            account_infos.push(self.#ctoken_config.to_account_info());
            account_infos.push(self.#ctoken_rent_sponsor.to_account_info());
            account_infos.push(self.#authority.to_account_info());
            account_infos.push(self.#mint_signer.to_account_info());
            account_infos.push(self.#fee_payer.to_account_info());

            let signer_seeds: &[&[u8]] = #signer_seeds_tokens;
            if signer_seeds.is_empty() {
                anchor_lang::solana_program::program::invoke(&mint_action_ix, &account_infos)?;
            } else {
                anchor_lang::solana_program::program::invoke_signed(&mint_action_ix, &account_infos, &[signer_seeds])?;
            }
        }

        Ok(true)
    }
}

/// Generate LightPreInit body for PDAs only (no mints)
/// After this, compressed addresses are registered
fn generate_pre_init_pdas_only(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) =
        generate_pda_compress_blocks(&parsed.rentfree_fields, params_ident);
    let rentfree_count = parsed.rentfree_fields.len() as u8;

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

        // Collect compressed infos for all rentfree accounts
        let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
        #(#compress_blocks)*

        // Execute Light System Program CPI directly with proof
        use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
        light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
            crate::LIGHT_CPI_SIGNER,
            #params_ident.create_accounts_proof.proof.clone()
        )
            .with_new_addresses(&[#(#new_addr_idents),*])
            .with_account_infos(&all_compressed_infos)
            .invoke(cpi_accounts)?;

        Ok(true)
    }
}

/// Generate compression blocks for PDA fields
fn generate_pda_compress_blocks(
    fields: &[RentFreeField],
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

        // Generate correct deref pattern: ** for Box<Account<T>>, * for Account<T>
        let deref_expr = if field.is_boxed {
            quote! { &mut **self.#ident }
        } else {
            quote! { &mut *self.#ident }
        };

        addr_idents.push(quote! { #new_addr_params_ident });

        blocks.push(quote! {
            // Get account info early before any mutable borrows
            let #account_info_ident = self.#ident.to_account_info();
            let #account_key_ident = #account_info_ident.key.to_bytes();

            let #new_addr_params_ident = {
                let tree_info = &#addr_tree_info;
                light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked {
                    seed: #account_key_ident,
                    address_merkle_tree_account_index: tree_info.address_merkle_tree_pubkey_index,
                    address_queue_account_index: tree_info.address_queue_pubkey_index,
                    address_merkle_tree_root_index: tree_info.root_index,
                    assigned_to_account: true,
                    assigned_account_index: #idx_lit,
                }
            };

            // Derive the compressed address
            let #address_ident = light_compressed_account::address::derive_address(
                &#new_addr_params_ident.seed,
                &cpi_accounts
                    .get_tree_account_info(#new_addr_params_ident.address_merkle_tree_account_index as usize)?
                    .key()
                    .to_bytes(),
                &crate::ID.to_bytes(),
            );

            // Get mutable reference to inner account data
            let #account_data_ident = #deref_expr;

            let #compressed_infos_ident = light_sdk::compressible::prepare_compressed_account_on_init::<#acc_ty_path>(
                &#account_info_ident,
                #account_data_ident,
                &compression_config_data,
                #address_ident,
                #new_addr_params_ident,
                #output_tree,
                &cpi_accounts,
                &compression_config_data.address_space,
                false, // at init, we do not compress_and_close the pda, we just "register" the empty compressed account with the derived address.
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
