use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Result, Token,
};

/// Simple version without lifetime issues
struct SimpleAccountTypeList {
    types: Vec<Ident>,
}

impl Parse for SimpleAccountTypeList {
    fn parse(input: ParseStream) -> Result<Self> {
        let punctuated: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(input)?;
        Ok(SimpleAccountTypeList {
            types: punctuated.into_iter().collect(),
        })
    }
}

/// Simple instruction generator that avoids lifetime issues
pub fn compressed_account_variant_with_instructions_simple(input: TokenStream) -> Result<TokenStream> {
    let type_list = syn::parse2::<SimpleAccountTypeList>(input)?;
    let account_types = &type_list.types;
    
    if account_types.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "At least one account type must be specified",
        ));
    }

    // Generate enum and traits by calling the existing variant_enum function directly
    let mut enum_input = TokenStream::new();
    for (i, account_type) in account_types.iter().enumerate() {
        if i > 0 {
            enum_input.extend(quote! { , });
        }
        enum_input.extend(quote! { #account_type });
    }
    
    // Call the existing variant_enum function
    let enum_and_traits = crate::variant_enum::compressed_account_variant(enum_input)?;
    
    // Generate simple decompress instruction without the complex seed derivation for now
    let decompress_instruction = generate_simple_decompress_instruction(account_types);
    let compress_instruction = generate_simple_compress_instruction(account_types);
    
    let expanded = quote! {
        #enum_and_traits
        #decompress_instruction
        #compress_instruction
    };

    Ok(expanded)
}

fn generate_simple_decompress_instruction(account_types: &[Ident]) -> TokenStream {
    // Generate match arms using the existing manual seed functions
    let decompress_match_arms = account_types.iter().map(|name| {
        match name.to_string().as_str() {
            "UserRecord" => quote! {
                CompressedAccountVariant::UserRecord(data) => {
                    let (seeds_vec, _) = get_user_record_seeds(&data.owner);
                    
                    let compressed_infos = light_sdk::compressible::prepare_account_for_decompression_idempotent::<UserRecord>(
                        &crate::ID,
                        data,
                        light_sdk::compressible::into_compressed_meta_with_address(
                            &compressed_data.meta,
                            &solana_accounts[i],
                            address_space,
                            &crate::ID,
                        ),
                        &solana_accounts[i],
                        &ctx.accounts.rent_payer,
                        &cpi_accounts,
                        seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>()
                            .as_slice(),
                    )?;
                    compressed_pda_infos.extend(compressed_infos);
                }
            },
            "GameSession" => quote! {
                CompressedAccountVariant::GameSession(data) => {
                    let (seeds_vec, _) = get_game_session_seeds(data.session_id);

                    let compressed_infos = light_sdk::compressible::prepare_account_for_decompression_idempotent::<GameSession>(
                        &crate::ID,
                        data,
                        light_sdk::compressible::into_compressed_meta_with_address(
                            &compressed_data.meta,
                            &solana_accounts[i],
                            address_space,
                            &crate::ID,
                        ),
                        &solana_accounts[i],
                        &ctx.accounts.rent_payer,
                        &cpi_accounts,
                        seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>()
                            .as_slice(),
                    )?;
                    compressed_pda_infos.extend(compressed_infos);
                }
            },
            "PlaceholderRecord" => quote! {
                CompressedAccountVariant::PlaceholderRecord(data) => {
                    let (seeds_vec, _) = get_placeholder_record_seeds(data.placeholder_id);

                    let compressed_infos = light_sdk::compressible::prepare_account_for_decompression_idempotent::<PlaceholderRecord>(
                        &crate::ID,
                        data,
                        light_sdk::compressible::into_compressed_meta_with_address(
                            &compressed_data.meta,
                            &solana_accounts[i],
                            address_space,
                            &crate::ID,
                        ),
                        &solana_accounts[i],
                        &ctx.accounts.rent_payer,
                        &cpi_accounts,
                        seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>()
                            .as_slice(),
                    )?;
                    compressed_pda_infos.extend(compressed_infos);
                }
            },
            _ => quote! {
                CompressedAccountVariant::#name(_) => {
                    return Err(anchor_lang::error::ErrorCode::InstructionDidNotDeserialize.into());
                }
            }
        }
    });

    let packed_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    quote! {
        /// Auto-generated decompress_accounts_idempotent instruction
        pub fn decompress_accounts_idempotent<'info>(
            ctx: anchor_lang::prelude::Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> anchor_lang::prelude::Result<()> {
            // Load config
            let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
            let address_space = compression_config.address_space[0];

            let (mut has_tokens, mut has_pdas) = (false, false);
            for c in &compressed_accounts {
                match c.data {
                    CompressedAccountVariant::CompressibleTokenAccountPacked(_) => has_tokens = true,
                    _ => has_pdas = true,
                }
                if has_tokens && has_pdas {
                    break;
                }
            }

            let cpi_accounts = if has_tokens && has_pdas {
                light_sdk_types::CpiAccountsSmall::new_with_config(
                    ctx.accounts.fee_payer.as_ref(),
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    light_sdk_types::CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
                )
            } else {
                light_sdk_types::CpiAccountsSmall::new(
                    ctx.accounts.fee_payer.as_ref(),
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    LIGHT_CPI_SIGNER,
                )
            };

            let pda_accounts_start = ctx.remaining_accounts.len() - compressed_accounts.len();
            let solana_accounts = &ctx.remaining_accounts[pda_accounts_start..];

            let mut compressed_token_accounts = Vec::new();
            let mut compressed_pda_infos = Vec::new();

            for (i, compressed_data) in compressed_accounts.clone().into_iter().enumerate() {
                let unpacked_data = compressed_data
                    .data
                    .unpack(cpi_accounts.post_system_accounts().unwrap())?;

                match unpacked_data {
                    #(#decompress_match_arms)*
                    #(#packed_match_arms)*
                    CompressedAccountVariant::CompressibleTokenAccountPacked(data) => {
                        compressed_token_accounts.push((data, compressed_data.meta));
                    }
                    CompressedAccountVariant::CompressibleTokenData(_) => {
                        unreachable!();
                    }
                }
            }

            // set new based on actually uninitialized accounts.
            let has_pdas = !compressed_pda_infos.is_empty();
            let has_tokens = !compressed_token_accounts.is_empty();
            if !has_pdas && !has_tokens {
                anchor_lang::prelude::msg!("All accounts already initialized.");
                return Ok(());
            }

            let fee_payer = ctx.accounts.fee_payer.as_ref();
            let authority = cpi_accounts.authority().unwrap();
            let cpi_context = cpi_accounts.cpi_context().unwrap();

            // First CPI.
            if has_pdas && has_tokens {
                let system_cpi_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
                    fee_payer,
                    authority,
                    cpi_context,
                    cpi_signer: LIGHT_CPI_SIGNER,
                };
                let cpi_inputs = light_sdk::cpi::CpiInputs::new_first_cpi(compressed_pda_infos, vec![]);
                cpi_inputs.invoke_light_system_program_cpi_context(system_cpi_accounts)?;
            } else if has_pdas {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, compressed_pda_infos);
                cpi_inputs.invoke_light_system_program_small(cpi_accounts.clone())?;
            }

            // Token decompression logic would go here (same as manual implementation)

            Ok(())
        }
    }
}

fn generate_simple_compress_instruction(account_types: &[Ident]) -> TokenStream {
    // For now, generate a simple placeholder
    quote! {
        /// Auto-generated compress_accounts_idempotent instruction (placeholder)
        pub fn compress_accounts_idempotent<'info>(
            ctx: anchor_lang::prelude::Context<'_, '_, 'info, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> anchor_lang::prelude::Result<()> {
            // Placeholder implementation
            Ok(())
        }
    }
}
