use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Item, ItemMod, Result, Token,
};

/// Parse a comma-separated list of identifiers
struct IdentList {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for IdentList {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Expected at least one account type",
            ));
        }

        // Try to parse as a simple identifier first
        if input.peek(Ident) && !input.peek2(Token![,]) {
            // Single identifier case
            let ident: Ident = input.parse()?;
            let mut idents = Punctuated::new();
            idents.push(ident);
            return Ok(IdentList { idents });
        }

        // Otherwise parse as comma-separated list
        Ok(IdentList {
            idents: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generate compress instructions for the specified account types (Native Solana version)
pub(crate) fn add_native_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    // Try to parse the arguments
    let ident_list = match syn::parse2::<IdentList>(args) {
        Ok(list) => list,
        Err(e) => {
            return Err(syn::Error::new(
                e.span(),
                format!("Failed to parse arguments: {}", e),
            ));
        }
    };

    // Check if module has content
    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }

    // Get the module content
    let content = module.content.as_mut().unwrap();

    // Collect all struct names
    let struct_names: Vec<_> = ident_list.idents.iter().collect();

    // Add necessary imports at the beginning
    let imports: Item = syn::parse_quote! {
        use super::*;
    };
    content.1.insert(0, imports);

    // Add borsh imports
    let borsh_imports: Item = syn::parse_quote! {
        use borsh::{BorshDeserialize, BorshSerialize};
    };
    content.1.insert(1, borsh_imports);

    // Generate unified data structures
    let unified_structures = generate_unified_structures(&struct_names);
    for item in unified_structures {
        content.1.push(item);
    }

    // Generate instruction data structures
    let instruction_data_structs = generate_instruction_data_structs(&struct_names);
    for item in instruction_data_structs {
        content.1.push(item);
    }

    // Generate thin wrapper processor functions
    let processor_functions = generate_thin_processors(&struct_names);
    for item in processor_functions {
        content.1.push(item);
    }

    Ok(quote! {
        #module
    })
}

fn generate_unified_structures(struct_names: &[&Ident]) -> Vec<Item> {
    let mut items = Vec::new();

    // Generate the CompressedAccountVariant enum
    let enum_variants = struct_names.iter().map(|name| {
        quote! {
            #name(#name)
        }
    });

    let compressed_variant_enum: Item = syn::parse_quote! {
        #[derive(Clone, Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
        pub enum CompressedAccountVariant {
            #(#enum_variants),*
        }
    };
    items.push(compressed_variant_enum);

    // Generate Default implementation
    if let Some(first_struct) = struct_names.first() {
        let default_impl: Item = syn::parse_quote! {
            impl Default for CompressedAccountVariant {
                fn default() -> Self {
                    CompressedAccountVariant::#first_struct(Default::default())
                }
            }
        };
        items.push(default_impl);
    }

    // Generate DataHasher implementation with correct signature
    let hash_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.hash::<H>()
        }
    });

    let data_hasher_impl: Item = syn::parse_quote! {
        impl light_hasher::DataHasher for CompressedAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> Result<[u8; 32], light_hasher::errors::HasherError> {
                match self {
                    #(#hash_match_arms),*
                }
            }
        }
    };
    items.push(data_hasher_impl);

    // Generate LightDiscriminator implementation with correct constants and method signature
    let light_discriminator_impl: Item = syn::parse_quote! {
        impl light_sdk::LightDiscriminator for CompressedAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // Default discriminator for enum
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;

            fn discriminator() -> [u8; 8] {
                Self::LIGHT_DISCRIMINATOR
            }
        }
    };
    items.push(light_discriminator_impl);

    // Generate HasCompressionInfo implementation with correct method signatures
    let compression_info_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.compression_info()
        }
    });

    let compression_info_mut_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.compression_info_mut()
        }
    });

    let has_compression_info_impl: Item = syn::parse_quote! {
        impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_match_arms),*
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_mut_match_arms),*
                }
            }
        }
    };
    items.push(has_compression_info_impl);

    // Generate CompressedAccountData struct
    let compressed_account_data: Item = syn::parse_quote! {
        #[derive(Clone, Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
        pub struct CompressedAccountData {
            pub meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            pub data: CompressedAccountVariant,
            pub seeds: Vec<Vec<u8>>, // Seeds for PDA derivation (without bump)
        }
    };
    items.push(compressed_account_data);

    items
}

fn generate_instruction_data_structs(struct_names: &[&Ident]) -> Vec<Item> {
    let mut items = Vec::new();

    // Create config instruction data
    let create_config: Item = syn::parse_quote! {
        #[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
        pub struct CreateCompressionConfigData {
            pub compression_delay: u32,
            pub rent_recipient: solana_program::pubkey::Pubkey,
            pub address_space: Vec<solana_program::pubkey::Pubkey>,
        }
    };
    items.push(create_config);

    // Update config instruction data
    let update_config: Item = syn::parse_quote! {
        #[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
        pub struct UpdateCompressionConfigData {
            pub new_compression_delay: Option<u32>,
            pub new_rent_recipient: Option<solana_program::pubkey::Pubkey>,
            pub new_address_space: Option<Vec<solana_program::pubkey::Pubkey>>,
            pub new_update_authority: Option<solana_program::pubkey::Pubkey>,
        }
    };
    items.push(update_config);

    // Decompress multiple PDAs instruction data
    let decompress_multiple: Item = syn::parse_quote! {
        #[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
        pub struct DecompressMultiplePdasData {
            pub proof: light_sdk::instruction::ValidityProof,
            pub compressed_accounts: Vec<CompressedAccountData>,
            pub bumps: Vec<u8>,
            pub system_accounts_offset: u8,
        }
    };
    items.push(decompress_multiple);

    // Generate compress instruction data for each struct
    for struct_name in struct_names {
        let compress_data_name = format_ident!("Compress{}Data", struct_name);
        let compress_data: Item = syn::parse_quote! {
            #[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
            pub struct #compress_data_name {
                pub proof: light_sdk::instruction::ValidityProof,
                pub compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            }
        };
        items.push(compress_data);
    }

    items
}

fn generate_thin_processors(struct_names: &[&Ident]) -> Vec<Item> {
    let mut functions = Vec::new();

    // Create config processor
    let create_config_fn: Item = syn::parse_quote! {
        /// Creates a compression config for this program
        ///
        /// Accounts expected:
        /// 0. `[writable, signer]` Payer account
        /// 1. `[writable]` Config PDA (seeds: [b"compressible_config"])
        /// 2. `[]` Program data account
        /// 3. `[signer]` Program upgrade authority
        /// 4. `[]` System program
        pub fn create_compression_config(
            accounts: &[solana_program::account_info::AccountInfo],
            compression_delay: u32,
            rent_recipient: solana_program::pubkey::Pubkey,
            address_space: Vec<solana_program::pubkey::Pubkey>,
        ) -> solana_program::entrypoint::ProgramResult {
            if accounts.len() < 5 {
                return Err(solana_program::program_error::ProgramError::NotEnoughAccountKeys);
            }

            let payer = &accounts[0];
            let config_account = &accounts[1];
            let program_data = &accounts[2];
            let authority = &accounts[3];
            let system_program = &accounts[4];

            light_sdk::compressible::create_compression_config_checked(
                config_account,
                authority,
                program_data,
                &rent_recipient,
                address_space,
                compression_delay,
                payer,
                system_program,
                &crate::ID,
            )
            .map_err(|e| solana_program::program_error::ProgramError::from(e))?;

            Ok(())
        }
    };
    functions.push(create_config_fn);

    // Update config processor
    let update_config_fn: Item = syn::parse_quote! {
        /// Updates the compression config
        ///
        /// Accounts expected:
        /// 0. `[writable]` Config PDA (seeds: [b"compressible_config"])
        /// 1. `[signer]` Update authority (must match config)
        pub fn update_compression_config(
            accounts: &[solana_program::account_info::AccountInfo],
            new_compression_delay: Option<u32>,
            new_rent_recipient: Option<solana_program::pubkey::Pubkey>,
            new_address_space: Option<Vec<solana_program::pubkey::Pubkey>>,
            new_update_authority: Option<solana_program::pubkey::Pubkey>,
        ) -> solana_program::entrypoint::ProgramResult {
            if accounts.len() < 2 {
                return Err(solana_program::program_error::ProgramError::NotEnoughAccountKeys);
            }

            let config_account = &accounts[0];
            let authority = &accounts[1];

            light_sdk::compressible::update_compression_config(
                config_account,
                authority,
                new_update_authority.as_ref(),
                new_rent_recipient.as_ref(),
                new_address_space,
                new_compression_delay,
                &crate::ID,
            )
            .map_err(|e| solana_program::program_error::ProgramError::from(e))?;

            Ok(())
        }
    };
    functions.push(update_config_fn);

    // Decompress multiple PDAs processor
    let variant_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => {
                CompressedAccountVariant::#name(data)
            }
        }
    });

    let decompress_fn: Item = syn::parse_quote! {
        /// Decompresses multiple compressed PDAs in a single transaction
        ///
        /// Accounts expected:
        /// 0. `[writable, signer]` Fee payer
        /// 1. `[writable, signer]` Rent payer
        /// 2. `[]` System program
        /// 3..N. `[writable]` PDA accounts to decompress into
        /// N+1... `[]` Light Protocol system accounts
        pub fn decompress_multiple_pdas(
            accounts: &[solana_program::account_info::AccountInfo],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            bumps: Vec<u8>,
            system_accounts_offset: u8,
        ) -> solana_program::entrypoint::ProgramResult {
            if accounts.len() < 3 {
                return Err(solana_program::program_error::ProgramError::NotEnoughAccountKeys);
            }

            let fee_payer = &accounts[0];
            let rent_payer = &accounts[1];

            // Get PDA accounts from remaining accounts
            let pda_accounts_end = system_accounts_offset as usize;
            let solana_accounts = &accounts[3..3 + pda_accounts_end];
            let system_accounts = &accounts[3 + pda_accounts_end..];

            // Validate we have matching number of PDAs, compressed accounts, and bumps
            if solana_accounts.len() != compressed_accounts.len()
                || solana_accounts.len() != bumps.len() {
                return Err(solana_program::program_error::ProgramError::InvalidAccountData);
            }

            let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                fee_payer,
                system_accounts,
                crate::LIGHT_CPI_SIGNER,
            );

            // Convert to unified enum accounts
            let mut light_accounts = Vec::new();
            let mut pda_account_refs = Vec::new();
            let mut signer_seeds_storage = Vec::new();

            for (i, (compressed_data, bump)) in compressed_accounts.into_iter()
                .zip(bumps.iter()).enumerate() {

                // Convert to unified enum type
                let unified_account = match compressed_data.data {
                    #(#variant_match_arms)*
                };

                let light_account = light_sdk::account::sha::LightAccount::<'_, CompressedAccountVariant>::new_mut(
                    &crate::ID,
                    &compressed_data.meta,
                    unified_account.clone(),
                )
                .map_err(|e| solana_program::program_error::ProgramError::from(e))?;

                // Build signer seeds based on account type
                let seeds = match &unified_account {
                    #(
                        CompressedAccountVariant::#struct_names(_) => {
                            // Get the seeds from the instruction data and append bump
                            let mut seeds = compressed_data.seeds.clone();
                            seeds.push(vec![*bump]);
                            seeds
                        }
                    ),*
                };

                signer_seeds_storage.push(seeds);
                light_accounts.push(light_account);
                pda_account_refs.push(&solana_accounts[i]);
            }

            // Convert to the format needed by the SDK
            let signer_seeds_refs: Vec<Vec<&[u8]>> = signer_seeds_storage
                .iter()
                .map(|seeds| seeds.iter().map(|s| s.as_slice()).collect())
                .collect();
            let signer_seeds_slices: Vec<&[&[u8]]> = signer_seeds_refs
                .iter()
                .map(|seeds| seeds.as_slice())
                .collect();

            // Single CPI call with unified enum type
            light_sdk::compressible::decompress_multiple_idempotent::<CompressedAccountVariant>(
                &pda_account_refs,
                light_accounts,
                &signer_seeds_slices,
                proof,
                cpi_accounts,
                &crate::ID,
                rent_payer,
            )
            .map_err(|e| solana_program::program_error::ProgramError::from(e))?;

            Ok(())
        }
    };
    functions.push(decompress_fn);

    // Generate compress processors for each account type
    for struct_name in struct_names {
        let compress_fn_name =
            format_ident!("compress_{}", struct_name.to_string().to_snake_case());

        let compress_processor: Item = syn::parse_quote! {
            /// Compresses a #struct_name PDA
            ///
            /// Accounts expected:
            /// 0. `[signer]` Authority
            /// 1. `[writable]` PDA account to compress
            /// 2. `[]` System program
            /// 3. `[]` Config PDA
            /// 4. `[]` Rent recipient (must match config)
            /// 5... `[]` Light Protocol system accounts
            pub fn #compress_fn_name(
                accounts: &[solana_program::account_info::AccountInfo],
                proof: light_sdk::instruction::ValidityProof,
                compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            ) -> solana_program::entrypoint::ProgramResult {
                if accounts.len() < 6 {
                    return Err(solana_program::program_error::ProgramError::NotEnoughAccountKeys);
                }

                let authority = &accounts[0];
                let solana_account = &accounts[1];
                let _system_program = &accounts[2];
                let config_account = &accounts[3];
                let rent_recipient = &accounts[4];
                let system_accounts = &accounts[5..];

                // Load config from AccountInfo
                let config = light_sdk::compressible::CompressibleConfig::load_checked(
                    config_account,
                    &crate::ID
                ).map_err(|_| solana_program::program_error::ProgramError::InvalidAccountData)?;

                // Verify rent recipient matches config
                if rent_recipient.key != &config.rent_recipient {
                    return Err(solana_program::program_error::ProgramError::InvalidAccountData);
                }

                let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                    authority,
                    system_accounts,
                    crate::LIGHT_CPI_SIGNER,
                );

                light_sdk::compressible::compress_account::<#struct_name>(
                    solana_account,
                    &compressed_account_meta,
                    proof,
                    cpi_accounts,
                    &crate::ID,
                    rent_recipient,
                    &config.compression_delay,
                )
                .map_err(|e| solana_program::program_error::ProgramError::from(e))?;

                Ok(())
            }
        };
        functions.push(compress_processor);
    }

    functions
}
