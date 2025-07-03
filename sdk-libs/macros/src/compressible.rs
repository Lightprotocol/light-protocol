use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    visit_mut, Attribute, Expr, Field, Ident, Item, ItemEnum, ItemFn, ItemMod, ItemStruct, Result,
    Token,
};

/// Parse a comma-separated list of identifiers
struct IdentList {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for IdentList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(IdentList {
            idents: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Information about seeds extracted from an account struct
#[derive(Debug, Clone)]
struct SeedInfo {
    seeds: Vec<Expr>,
    bump_field: Option<Ident>,
}

/// Extract instruction parameter names from #[instruction(...)] attribute
fn extract_instruction_param_names(attrs: &[Attribute]) -> Vec<String> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let mut param_names = Vec::new();
            let _ = attr.parse_nested_meta(|meta| {
                // Extract the parameter name from the path
                if let Some(ident) = meta.path.get_ident() {
                    param_names.push(ident.to_string());
                }
                // Skip the type if present (after colon)
                if meta.input.peek(Token![:]) {
                    meta.input.parse::<Token![:]>()?;
                    meta.input.parse::<syn::Type>()?;
                }
                Ok(())
            });
            if !param_names.is_empty() {
                return param_names;
            }
        }
    }
    vec!["account_data".to_string()] // Default fallback
}

/// Check if a struct has the Accounts derive using proper AST parsing
fn has_accounts_derive(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            let mut has_accounts = false;
            let _ = attr.parse_nested_meta(|meta| {
                // Check if this derive item is "Accounts" or ends with "::Accounts"
                if let Some(ident) = meta.path.get_ident() {
                    if ident == "Accounts" {
                        has_accounts = true;
                    }
                } else if let Some(last_segment) = meta.path.segments.last() {
                    if last_segment.ident == "Accounts" {
                        has_accounts = true;
                    }
                }
                Ok(())
            });
            has_accounts
        } else {
            false
        }
    })
}

/// Scan module items to find account structs that initialize the given account type
fn find_account_seeds_for_type(
    module_items: &[Item],
    account_type: &Ident,
) -> Result<Option<SeedInfo>> {
    for item in module_items {
        if let Item::Struct(item_struct) = item {
            // Check if this struct has Accounts derive
            let has_accounts_derive = has_accounts_derive(&item_struct.attrs);

            if !has_accounts_derive {
                continue;
            }

            // Get instruction parameter names from this struct
            let _param_names = extract_instruction_param_names(&item_struct.attrs);

            // Look for a field of our target account type with init constraint
            if let syn::Fields::Named(fields) = &item_struct.fields {
                for field in &fields.named {
                    if let Some(seeds_info) = extract_seeds_from_field(field, account_type)? {
                        return Ok(Some(seeds_info));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Check if a type matches Account<'info, TargetType> with robust handling of qualified paths
fn matches_account_type(ty: &syn::Type, target_type: &Ident) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(last_segment) = type_path.path.segments.last() {
                // Handle Account, AccountLoader, or other account wrapper types
                let account_type_names = ["Account", "AccountLoader", "InterfaceAccount"];
                if account_type_names.contains(&&*last_segment.ident.to_string()) {
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        // Look for the account type in generic arguments (usually second after lifetime)
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(syn::Type::Path(inner_type)) = arg {
                                // Check if this type path matches our target
                                if let Some(inner_segment) = inner_type.path.segments.last() {
                                    if inner_segment.ident == *target_type {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Parse account attribute to extract init, seeds, and bump information using proper AST parsing
fn parse_account_attribute(attr: &Attribute) -> Result<Option<(bool, Vec<Expr>, bool)>> {
    if !attr.path().is_ident("account") {
        return Ok(None);
    }

    let mut has_init = false;
    let mut seeds = Vec::new();
    let mut has_bump = false;

    // Parse the attribute content
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("init") {
            has_init = true;
            Ok(())
        } else if meta.path.is_ident("bump") {
            has_bump = true;
            Ok(())
        } else if meta.path.is_ident("seeds") {
            // Parse seeds = [...]
            if meta.input.peek(Token![=]) {
                meta.input.parse::<Token![=]>()?; // Consume the equals sign
                let content;
                bracketed!(content in meta.input);
                let seed_exprs: Punctuated<Expr, Token![,]> =
                    content.parse_terminated(Expr::parse, Token![,])?;
                seeds = seed_exprs.into_iter().collect();
            }
            Ok(())
        } else {
            // Skip other attributes like payer, space, etc.
            if meta.input.peek(Token![=]) {
                meta.input.parse::<Token![=]>()?;
                meta.input.parse::<Expr>()?;
            }
            Ok(())
        }
    })?;

    Ok(Some((has_init, seeds, has_bump)))
}

/// Convert instruction parameter references in seeds to account field references
fn convert_seed_parameters(seeds: Vec<Expr>, target_type: &Ident) -> Result<Vec<Expr>> {
    let mut converted_seeds = Vec::new();

    for seed in seeds {
        let converted = convert_single_seed_parameter(seed, target_type)?;
        converted_seeds.push(converted);
    }

    Ok(converted_seeds)
}

/// Convert a single seed expression from instruction parameter to account field reference
fn convert_single_seed_parameter(seed: Expr, _target_type: &Ident) -> Result<Expr> {
    // Use visitor pattern to find and replace parameter references
    struct ParameterConverter {
        converted: bool,
    }

    impl visit_mut::VisitMut for ParameterConverter {
        fn visit_expr_field_mut(&mut self, field_expr: &mut syn::ExprField) {
            // Look for expressions like account_data.field or similar parameter patterns
            if let syn::Expr::Path(base_path) = field_expr.base.as_ref() {
                if let Some(ident) = base_path.path.get_ident() {
                    let ident_str = ident.to_string();
                    // Check for various parameter naming patterns
                    if ident_str.ends_with("_data")
                        || ident_str == "account_data"
                        || ident_str == "params"
                    {
                        // Replace with solana_account
                        *field_expr.base = syn::parse_quote!(solana_account);
                        self.converted = true;
                    }
                }
            }

            // Continue visiting nested expressions
            visit_mut::visit_expr_field_mut(self, field_expr);
        }
    }

    let mut seed_copy = seed;
    let mut converter = ParameterConverter { converted: false };
    visit_mut::visit_expr_mut(&mut converter, &mut seed_copy);

    Ok(seed_copy)
}

/// Extract seeds from a field's account attribute if it matches the target type and has init constraint
fn extract_seeds_from_field(field: &Field, target_type: &Ident) -> Result<Option<SeedInfo>> {
    // Check if field type matches target type
    let field_type_matches = matches_account_type(&field.ty, target_type);

    if !field_type_matches {
        return Ok(None);
    }

    // Look for account attribute with init and seeds
    for attr in &field.attrs {
        if let Some((has_init, seeds, has_bump)) = parse_account_attribute(attr)? {
            if has_init && !seeds.is_empty() {
                let bump_field = if has_bump {
                    Some(format_ident!("bump"))
                } else {
                    None
                };

                // Convert instruction parameter references to account field references
                let converted_seeds = convert_seed_parameters(seeds, target_type)?;

                return Ok(Some(SeedInfo {
                    seeds: converted_seeds,
                    bump_field,
                }));
            }
        }
    }

    Ok(None)
}

/// Generate compress instructions for the specified account types (Anchor version)
pub(crate) fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    let ident_list = syn::parse2::<IdentList>(args)?;

    // Check if module has content
    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }

    // Get the module content
    let content = module.content.as_mut().unwrap();

    // Collect all struct names for the enum
    let struct_names: Vec<_> = ident_list.idents.iter().cloned().collect();

    // Generate the CompressedAccountVariant enum
    let enum_variants = struct_names.iter().map(|name| {
        quote! { #name(#name) }
    });

    let compressed_account_variant_enum: ItemEnum = syn::parse_quote! {
        #[derive(Clone, Debug, light_sdk::AnchorSerialize, light_sdk::AnchorDeserialize)]
        pub enum CompressedAccountVariant {
            #(#enum_variants),*
        }
    };

    // Generate Default implementation for the enum
    if struct_names.is_empty() {
        return Err(syn::Error::new_spanned(
            &module,
            "At least one account struct must be specified",
        ));
    }

    let first_struct = struct_names.first().expect("At least one struct required");
    let default_impl: Item = syn::parse_quote! {
        impl Default for CompressedAccountVariant {
            fn default() -> Self {
                CompressedAccountVariant::#first_struct(Default::default())
            }
        }
    };

    // Generate DataHasher implementation for the enum
    let hash_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.hash::<H>()
        }
    });

    let data_hasher_impl: Item = syn::parse_quote! {
        impl light_hasher::DataHasher for CompressedAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::errors::HasherError> {
                match self {
                    #(#hash_match_arms),*
                }
            }
        }
    };

    // Generate LightDiscriminator implementation for the enum
    let light_discriminator_impl: Item = syn::parse_quote! {
        impl light_sdk::LightDiscriminator for CompressedAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // This won't be used directly
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    // Generate HasCompressionInfo implementation for the enum
    let has_compression_info_impl: Item = syn::parse_quote! {
        impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info()),*
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info_mut()),*
                }
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info_mut_opt()),*
                }
            }

            fn set_compression_info_none(&mut self) {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.set_compression_info_none()),*
                }
            }
        }
    };

    // Generate Size implementation for the enum
    let size_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.size()
        }
    });

    let size_impl: Item = syn::parse_quote! {
        impl light_sdk::Size for CompressedAccountVariant {
            fn size(&self) -> usize {
                match self {
                    #(#size_match_arms),*
                }
            }
        }
    };

    // Generate the CompressedAccountData struct
    let compressed_account_data: ItemStruct = syn::parse_quote! {
        #[derive(Clone, Debug, light_sdk::AnchorDeserialize, light_sdk::AnchorSerialize)]
        pub struct CompressedAccountData {
            pub meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            pub data: CompressedAccountVariant,
            pub seeds: Vec<Vec<u8>>, // Seeds for PDA derivation (without bump)
        }
    };

    // Generate config-related structs and instructions
    let initialize_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct InitializeCompressionConfig<'info> {
            #[account(mut)]
            pub payer: Signer<'info>,
            /// The config PDA to be created
            /// CHECK: Config PDA is checked by the SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// The program's data account
            /// CHECK: Program data account is validated by the SDK
            pub program_data: AccountInfo<'info>,
            /// The program's upgrade authority (must sign)
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
    };

    // Generate the update_compression_config accounts struct
    let update_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct UpdateCompressionConfig<'info> {
            /// CHECK: Config is checked by the SDK's load_checked method
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// Must match the update authority stored in config
            pub authority: Signer<'info>,
        }
    };

    let initialize_compression_config_fn: ItemFn = syn::parse_quote! {
        /// Create compressible config - only callable by program upgrade authority
        pub fn initialize_compression_config(
            ctx: Context<InitializeCompressionConfig>,
            compression_delay: u32,
            rent_recipient: Pubkey,
            address_space: Vec<Pubkey>,
            config_bump: Option<u8>,
        ) -> anchor_lang::Result<()> {
            let config_bump = config_bump.unwrap_or(0);
            light_sdk::compressible::process_initialize_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_recipient,
                address_space,
                compression_delay,
                config_bump,
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            )?;

            Ok(())
        }
    };

    let update_compression_config_fn: ItemFn = syn::parse_quote! {
        /// Update compressible config - only callable by config's update authority
        pub fn update_compression_config(
            ctx: Context<UpdateCompressionConfig>,
            new_compression_delay: Option<u32>,
            new_rent_recipient: Option<Pubkey>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> anchor_lang::Result<()> {
            light_sdk::compressible::process_update_compression_config(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                new_update_authority.as_ref(),
                new_rent_recipient.as_ref(),
                new_address_space,
                new_compression_delay,
                &crate::ID,
            )?;

            Ok(())
        }
    };

    // Generate the decompress_accounts_idempotent accounts struct
    let decompress_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// UNCHECKED: Anyone can pay to init.
            #[account(mut)]
            pub rent_payer: Signer<'info>,
            /// The global config account
            /// CHECK: load_checked.
            pub config: AccountInfo<'info>,
            // Remaining accounts:
            // - First N accounts: PDA accounts to decompress into
            // - After system_accounts_offset: Light Protocol system accounts for CPI
        }
    };

    // Generate the decompress_accounts_idempotent instruction
    let decompress_instruction: ItemFn = syn::parse_quote! {
        /// Decompresses multiple compressed PDAs of any supported account type in a single transaction
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            bumps: Vec<u8>,
            system_accounts_offset: u8,
        ) -> anchor_lang::Result<()> {
            // Get PDA accounts from remaining accounts
            let pda_accounts_end = system_accounts_offset as usize;
            let solana_accounts = &ctx.remaining_accounts[..pda_accounts_end];

            // Validate we have matching number of PDAs, compressed accounts, and bumps
            if solana_accounts.len() != compressed_accounts.len() || solana_accounts.len() != bumps.len() {
                return err!(ErrorCode::InvalidAccountCount);
            }

            let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                &ctx.accounts.fee_payer,
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // Get address space from config checked.
            let config = light_sdk::compressible::CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
            let address_space = config.address_space[0];

            let mut all_compressed_infos = Vec::with_capacity(compressed_accounts.len());

            for (i, (compressed_data, &bump)) in compressed_accounts
                .into_iter()
                .zip(bumps.iter())
                .enumerate()
            {
                let bump_slice = [bump];

                match compressed_data.data {
                    #(
                        CompressedAccountVariant::#struct_names(data) => {
                            let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                            for seed in &compressed_data.seeds {
                                seeds_refs.push(seed.as_slice());
                            }
                            seeds_refs.push(&bump_slice);

                            // Create LightAccount with correct discriminator
                            let light_account = light_sdk::account::LightAccount::<'_, #struct_names>::new_mut(
                                &crate::ID,
                                &compressed_data.meta,
                                data,
                            )?;

                            // Process this single account
                            let compressed_infos = light_sdk::compressible::prepare_accounts_for_decompress_idempotent::<#struct_names>(
                                &[&solana_accounts[i]],
                                vec![light_account],
                                &[seeds_refs.as_slice()],
                                &cpi_accounts,
                                &ctx.accounts.rent_payer,
                                address_space,
                            )?;

                            all_compressed_infos.extend(compressed_infos);
                        }
                    ),*
                }
            }

            if all_compressed_infos.is_empty() {
                msg!("No compressed accounts to decompress");
            } else {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, all_compressed_infos);
                cpi_inputs.invoke_light_system_program(cpi_accounts)?;
            }

            Ok(())
        }
    };

    // Generate error code enum if it doesn't exist
    let error_code: Item = syn::parse_quote! {
        #[error_code]
        pub enum ErrorCode {
            #[msg("Invalid account count: PDAs and compressed accounts must match")]
            InvalidAccountCount,
            #[msg("Rent recipient does not match config")]
            InvalidRentRecipient,
        }
    };

    // Add all generated items to the module
    content.1.push(Item::Enum(compressed_account_variant_enum));
    content.1.push(default_impl);
    content.1.push(data_hasher_impl);
    content.1.push(light_discriminator_impl);
    content.1.push(has_compression_info_impl);
    content.1.push(size_impl);
    content.1.push(Item::Struct(compressed_account_data));
    content.1.push(Item::Struct(initialize_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(initialize_compression_config_fn));
    content.1.push(Item::Fn(update_compression_config_fn));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(error_code);

    // Generate compress instructions for each struct (NOT create instructions - those need custom logic)
    for struct_name in ident_list.idents {
        let compress_fn_name =
            format_ident!("compress_{}", struct_name.to_string().to_snake_case());
        let compress_accounts_name = format_ident!("Compress{}", struct_name);

        // Find seeds for this account type from existing account structs
        let seeds_info = find_account_seeds_for_type(&content.1, &struct_name)?
            .ok_or_else(|| syn::Error::new_spanned(
                &struct_name,
                format!(
                    "No account struct found with 'init' constraint and seeds for type '{}'. \
                    Please ensure you have an account struct (with #[derive(Accounts)]) that initializes \
                    this account type with seeds specified in the #[account(init, seeds = [...], ...)] attribute.",
                    struct_name
                )
            ))?;

        let seeds_expr = &seeds_info.seeds;
        let bump_constraint = if seeds_info.bump_field.is_some() {
            quote! { bump, }
        } else {
            quote! {}
        };

        // Generate the compress accounts struct with extracted seeds
        let compress_accounts_struct: ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct #compress_accounts_name<'info> {
                #[account(mut)]
                pub user: Signer<'info>,
                #[account(
                    mut,
                    seeds = [#(#seeds_expr),*],
                    #bump_constraint
                )]
                pub solana_account: Account<'info, #struct_name>,
                /// The global config account
                /// CHECK: load_checked.
                pub config: AccountInfo<'info>,
                /// Rent recipient - validated against config
                pub rent_recipient: AccountInfo<'info>,
            }
        };

        // Generate the compress instruction function
        let compress_instruction_fn: ItemFn = syn::parse_quote! {
            /// Compresses a #struct_name PDA using config values
            pub fn #compress_fn_name<'info>(
                ctx: Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                proof: light_sdk::instruction::ValidityProof,
                compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            ) -> anchor_lang::Result<()> {
                // Load config from AccountInfo
                let config = light_sdk::compressible::CompressibleConfig::load_checked(
                    &ctx.accounts.config,
                    &crate::ID
                ).map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

                // Verify rent recipient matches config
                if ctx.accounts.rent_recipient.key() != config.rent_recipient {
                    return err!(ErrorCode::InvalidRentRecipient);
                }

                let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                    &ctx.accounts.user,
                    &ctx.remaining_accounts[..],
                    LIGHT_CPI_SIGNER,
                );

                light_sdk::compressible::compress_account::<#struct_name>(
                    &mut ctx.accounts.solana_account,
                    &compressed_account_meta,
                    proof,
                    cpi_accounts,
                    &ctx.accounts.rent_recipient,
                    &config.compression_delay,
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

                Ok(())
            }
        };

        // Generate Size implementation for the struct
        let size_impl: Item = syn::parse_quote! {
            impl light_sdk::Size for #struct_name {
                fn size(&self) -> usize {
                    Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
                }
            }
        };

        // Add the generated items to the module (only compress, not create)
        content.1.push(Item::Struct(compress_accounts_struct));
        content.1.push(Item::Fn(compress_instruction_fn));
        content.1.push(size_impl);
    }

    Ok(quote! {
        #module
    })
}

/// Generates HasCompressionInfo trait implementation for a struct with compression_info field
pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = input.ident.clone();

    // Find the compression_info field
    let compression_info_field = match &input.fields {
        syn::Fields::Named(fields) => fields.named.iter().find(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| ident == "compression_info")
                .unwrap_or(false)
        }),
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_name,
                "HasCompressionInfo can only be derived for structs with named fields",
            ))
        }
    };

    let _compression_info_field = compression_info_field.ok_or_else(|| {
        syn::Error::new_spanned(
            &struct_name,
            "HasCompressionInfo requires a field named 'compression_info' of type Option<CompressionInfo>"
        )
    })?;

    // Validate that the field is Option<CompressionInfo>
    // For now, we'll assume it's correct and let the compiler catch type errors

    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_ref()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_mut()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                &mut self.compression_info
            }

            fn set_compression_info_none(&mut self) {
                self.compression_info = None;
            }
        }
    };

    Ok(has_compression_info_impl)
}
