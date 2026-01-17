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

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::light_mint::{generate_mint_action_invocation, MintActionConfig};
use super::parse::{ParsedRentFreeStruct, RentFreeField};

/// Resolve optional field name to TokenStream, using default if None
fn resolve_field_name(field: &Option<syn::Ident>, default: &str) -> TokenStream {
    field.as_ref().map(|f| quote! { #f }).unwrap_or_else(|| {
        let ident = format_ident!("{}", default);
        quote! { #ident }
    })
}

/// Generate both trait implementations.
///
/// Returns `Err` if the parsed struct has inconsistent state (e.g., params type without ident).
pub(super) fn generate_rentfree_impl(
    parsed: &ParsedRentFreeStruct,
) -> Result<TokenStream, syn::Error> {
    let struct_name = &parsed.struct_name;
    let (impl_generics, ty_generics, where_clause) = parsed.generics.split_for_impl();

    // Validation: Ensure combined PDA + mint count fits in u8 (Light Protocol uses u8 for account indices)
    let total_accounts = parsed.rentfree_fields.len() + parsed.light_mint_fields.len();
    if total_accounts > 255 {
        return Err(syn::Error::new_spanned(
            struct_name,
            format!(
                "Too many compression fields ({} PDAs + {} mints = {} total, maximum 255). \
                 Light Protocol uses u8 for account indices.",
                parsed.rentfree_fields.len(),
                parsed.light_mint_fields.len(),
                total_accounts
            ),
        ));
    }

    // Extract first instruction arg or generate no-op impls
    let first_arg = match parsed.instruction_args.as_ref().and_then(|args| args.first()) {
        Some(arg) => arg,
        None => {
            // No instruction args - generate no-op impls.
            // Keep these for backwards compatibility with structs that derive RentFree
            // without compression fields or instruction params.
            return Ok(quote! {
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
            });
        }
    };

    let params_type = &first_arg.ty;
    let params_ident = &first_arg.name;

    let has_pdas = !parsed.rentfree_fields.is_empty();
    let has_mints = !parsed.light_mint_fields.is_empty();

    // Resolve field names with defaults
    let fee_payer = resolve_field_name(&parsed.fee_payer_field, "fee_payer");
    let compression_config =
        resolve_field_name(&parsed.compression_config_field, "compression_config");
    let ctoken_config =
        resolve_field_name(&parsed.ctoken_config_field, "ctoken_compressible_config");
    let ctoken_rent_sponsor =
        resolve_field_name(&parsed.ctoken_rent_sponsor_field, "ctoken_rent_sponsor");
    let light_token_program =
        resolve_field_name(&parsed.ctoken_program_field, "light_token_program");
    let ctoken_cpi_authority =
        resolve_field_name(&parsed.ctoken_cpi_authority_field, "ctoken_cpi_authority");

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
            &light_token_program,
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
            &light_token_program,
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

    Ok(quote! {
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
    })
}

/// Generate LightPreInit body for PDAs + mints:
/// 1. Write PDAs to CPI context
/// 2. Invoke mint_action with decompress + CPI context
///    After this, Mint is "hot" and usable in instruction body
#[allow(clippy::too_many_arguments)]
fn generate_pre_init_pdas_and_mints(
    parsed: &ParsedRentFreeStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
    ctoken_config: &TokenStream,
    ctoken_rent_sponsor: &TokenStream,
    light_token_program: &TokenStream,
    ctoken_cpi_authority: &TokenStream,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) = generate_pda_compress_blocks(&parsed.rentfree_fields);
    let rentfree_count = parsed.rentfree_fields.len() as u8;
    let pda_count = parsed.rentfree_fields.len();

    // Get the first PDA's output tree index (for the state tree output queue)
    let first_pda_output_tree = &parsed.rentfree_fields[0].output_tree;

    // TODO(diff-pr): Support multiple #[light_mint] fields by looping here.
    // Each mint would get assigned_account_index = pda_count + mint_index.
    // Also add support for #[rentfree_token] fields for token ATAs.
    let mint = &parsed.light_mint_fields[0];

    // assigned_account_index for mint is after PDAs
    let mint_assigned_index = pda_count as u8;

    // Generate mint action invocation with CPI context
    let mint_invocation = generate_mint_action_invocation(&MintActionConfig {
        mint,
        params_ident,
        fee_payer,
        ctoken_config,
        ctoken_rent_sponsor,
        light_token_program,
        ctoken_cpi_authority,
        cpi_context: Some((quote! { #first_pda_output_tree }, mint_assigned_index)),
    });

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
        #mint_invocation

        Ok(true)
    }
}

/// Generate LightPreInit body for mints-only (no PDAs):
/// Invoke mint_action with decompress directly
/// After this, CMint is "hot" and usable in instruction body
#[allow(clippy::too_many_arguments)]
fn generate_pre_init_mints_only(
    parsed: &ParsedRentFreeStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    ctoken_config: &TokenStream,
    ctoken_rent_sponsor: &TokenStream,
    light_token_program: &TokenStream,
    ctoken_cpi_authority: &TokenStream,
) -> TokenStream {
    // TODO(diff-pr): Support multiple #[light_mint] fields by looping here.
    // Each mint would get assigned_account_index = mint_index.
    // Also add support for #[rentfree_token] fields for token ATAs.
    let mint = &parsed.light_mint_fields[0];

    // Generate mint action invocation without CPI context
    let mint_invocation = generate_mint_action_invocation(&MintActionConfig {
        mint,
        params_ident,
        fee_payer,
        ctoken_config,
        ctoken_rent_sponsor,
        light_token_program,
        ctoken_cpi_authority,
        cpi_context: None,
    });

    quote! {
        // Build CPI accounts (no CPI context needed for mints-only)
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
            &self.#fee_payer,
            _remaining,
            crate::LIGHT_CPI_SIGNER,
        );

        // Build and invoke mint_action with decompress
        #mint_invocation

        Ok(true)
    }
}

/// Generate LightPreInit body for PDAs only (no mints)
/// After this, compressed addresses are registered
fn generate_pre_init_pdas_only(
    parsed: &ParsedRentFreeStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) = generate_pda_compress_blocks(&parsed.rentfree_fields);
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
fn generate_pda_compress_blocks(fields: &[RentFreeField]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut blocks = Vec::new();
    let mut addr_idents = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let idx_lit = idx as u8;
        let ident = &field.ident;
        let addr_tree_info = &field.address_tree_info;
        let output_tree = &field.output_tree;
        let inner_type = &field.inner_type;

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

            let #compressed_infos_ident = light_sdk::compressible::prepare_compressed_account_on_init::<#inner_type>(
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
