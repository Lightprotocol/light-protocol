//! Parsing logic for #[rentfree(...)] and #[light_mint(...)] attributes.

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Error, Expr, Ident, Token, Type,
};

// Import shared types from anchor_seeds module
pub(super) use crate::compressible::anchor_seeds::extract_account_inner_type;

/// Parsed representation of a struct with rentfree and light_mint fields.
pub(super) struct ParsedCompressibleStruct {
    pub struct_name: Ident,
    pub generics: syn::Generics,
    pub rentfree_fields: Vec<RentFreeField>,
    pub light_mint_fields: Vec<LightMintField>,
    pub instruction_args: Option<Vec<InstructionArg>>,
    pub fee_payer_field: Option<Ident>,
    pub compression_config_field: Option<Ident>,
    /// CToken compressible config account (for decompress mint)
    pub ctoken_config_field: Option<Ident>,
    /// CToken rent sponsor account (for decompress mint)
    pub ctoken_rent_sponsor_field: Option<Ident>,
    /// CToken program account (for decompress mint CPI)
    pub ctoken_program_field: Option<Ident>,
    /// CToken CPI authority PDA (for decompress mint CPI)
    pub ctoken_cpi_authority_field: Option<Ident>,
}

/// A field marked with #[rentfree(...)]
pub(super) struct RentFreeField {
    pub ident: Ident,
    pub ty: Type,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
}

/// A field marked with #[light_mint(...)]
pub(super) struct LightMintField {
    /// The field name where #[light_mint] is attached (CMint account)
    pub field_ident: Ident,
    /// The mint_signer field (AccountInfo that seeds the mint PDA)
    pub mint_signer: Expr,
    /// The authority for mint operations
    pub authority: Expr,
    /// Decimals for the mint
    pub decimals: Expr,
    /// Address tree info expression
    pub address_tree_info: Expr,
    /// Optional freeze authority
    pub freeze_authority: Option<Expr>,
    /// Signer seeds for the mint_signer PDA (required if mint_signer is a PDA)
    pub signer_seeds: Option<Expr>,
    /// Rent payment epochs for decompression (default: 2)
    pub rent_payment: Option<Expr>,
    /// Write top-up lamports for decompression (default: 0)
    pub write_top_up: Option<Expr>,
}

/// Instruction argument from #[instruction(...)]
pub(super) struct InstructionArg {
    pub name: Ident,
    pub ty: Type,
}

impl Parse for InstructionArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(Self { name, ty })
    }
}

/// Arguments inside #[rentfree(...)]
struct RentFreeArgs {
    address_tree_info: Option<Expr>,
    output_tree: Option<Expr>,
}

impl Parse for RentFreeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = RentFreeArgs {
            address_tree_info: None,
            output_tree: None,
        };

        let content: Punctuated<KeyValueArg, Token![,]> = Punctuated::parse_terminated(input)?;

        for arg in content {
            match arg.name.to_string().as_str() {
                "address_tree_info" => args.address_tree_info = Some(arg.value),
                "output_tree" => args.output_tree = Some(arg.value),
                other => {
                    return Err(Error::new(
                        arg.name.span(),
                        format!("unknown rentfree attribute: {}", other),
                    ))
                }
            }
        }

        Ok(args)
    }
}

/// Arguments inside #[light_mint(...)]
struct LightMintArgs {
    mint_signer: Option<Expr>,
    authority: Option<Expr>,
    decimals: Option<Expr>,
    address_tree_info: Option<Expr>,
    freeze_authority: Option<Expr>,
    signer_seeds: Option<Expr>,
    rent_payment: Option<Expr>,
    write_top_up: Option<Expr>,
}

impl Parse for LightMintArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = LightMintArgs {
            mint_signer: None,
            authority: None,
            decimals: None,
            address_tree_info: None,
            freeze_authority: None,
            signer_seeds: None,
            rent_payment: None,
            write_top_up: None,
        };

        let content: Punctuated<KeyValueArg, Token![,]> = Punctuated::parse_terminated(input)?;

        for arg in content {
            match arg.name.to_string().as_str() {
                "mint_signer" => args.mint_signer = Some(arg.value),
                "authority" => args.authority = Some(arg.value),
                "decimals" => args.decimals = Some(arg.value),
                "address_tree_info" => args.address_tree_info = Some(arg.value),
                "freeze_authority" => args.freeze_authority = Some(arg.value),
                "signer_seeds" => args.signer_seeds = Some(arg.value),
                "rent_payment" => args.rent_payment = Some(arg.value),
                "write_top_up" => args.write_top_up = Some(arg.value),
                other => {
                    return Err(Error::new(
                        arg.name.span(),
                        format!("unknown light_mint attribute: {}", other),
                    ))
                }
            }
        }

        Ok(args)
    }
}

/// Generic key = value argument parser
struct KeyValueArg {
    name: Ident,
    value: Expr,
}

impl Parse for KeyValueArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(KeyValueArg { name, value })
    }
}

/// Parse #[instruction(...)] attribute from struct.
///
/// Returns `Ok(None)` if no instruction attribute is present,
/// `Ok(Some(args))` if successfully parsed, or `Err` on malformed syntax.
fn parse_instruction_attr(attrs: &[syn::Attribute]) -> Result<Option<Vec<InstructionArg>>, Error> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let args = attr.parse_args_with(|input: ParseStream| {
                let content: Punctuated<InstructionArg, Token![,]> =
                    Punctuated::parse_terminated(input)?;
                Ok(content.into_iter().collect::<Vec<_>>())
            })?;
            return Ok(Some(args));
        }
    }
    Ok(None)
}

/// Parse a struct to extract rentfree and light_mint fields
pub(super) fn parse_compressible_struct(
    input: &DeriveInput,
) -> Result<ParsedCompressibleStruct, Error> {
    let struct_name = input.ident.clone();
    let generics = input.generics.clone();

    let instruction_args = parse_instruction_attr(&input.attrs)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new_spanned(input, "expected named fields")),
        },
        _ => return Err(Error::new_spanned(input, "expected struct")),
    };

    let mut rentfree_fields = Vec::new();
    let mut light_mint_fields = Vec::new();
    let mut fee_payer_field = None;
    let mut compression_config_field = None;
    let mut ctoken_config_field = None;
    let mut ctoken_rent_sponsor_field = None;
    let mut ctoken_program_field = None;
    let mut ctoken_cpi_authority_field = None;

    for field in fields {
        let field_ident = field.ident.clone().ok_or_else(|| {
            Error::new_spanned(field, "expected named field with identifier")
        })?;
        let field_name = field_ident.to_string();

        // Track special fields by naming convention.
        //
        // The RentFree derive expects these conventional field names:
        //
        // Fee payer (who pays transaction fees and rent):
        //   - "fee_payer" (preferred), "payer", "creator"
        //
        // Compression config (holds compression settings for the program):
        //   - "compression_config"
        //
        // CToken fields (for compressed token mint operations):
        //   - Config: "ctoken_compressible_config", "ctoken_config", "light_token_config_account"
        //   - Rent sponsor: "ctoken_rent_sponsor", "light_token_rent_sponsor"
        //   - Program: "ctoken_program", "light_token_program"
        //   - CPI authority: "ctoken_cpi_authority", "light_token_program_cpi_authority",
        //                    "compress_token_program_cpi_authority"
        //
        // Fields not matching these names will use defaults in code generation.
        if field_name == "fee_payer" || field_name == "payer" || field_name == "creator" {
            fee_payer_field = Some(field_ident.clone());
        }
        if field_name == "compression_config" {
            compression_config_field = Some(field_ident.clone());
        }
        if field_name == "ctoken_compressible_config"
            || field_name == "ctoken_config"
            || field_name == "light_token_config_account"
        {
            ctoken_config_field = Some(field_ident.clone());
        }
        if field_name == "ctoken_rent_sponsor" || field_name == "light_token_rent_sponsor" {
            ctoken_rent_sponsor_field = Some(field_ident.clone());
        }
        if field_name == "ctoken_program" || field_name == "light_token_program" {
            ctoken_program_field = Some(field_ident.clone());
        }
        if field_name == "ctoken_cpi_authority"
            || field_name == "light_token_program_cpi_authority"
            || field_name == "compress_token_program_cpi_authority"
        {
            ctoken_cpi_authority_field = Some(field_ident.clone());
        }

        // Look for #[rentfree] or #[rentfree(...)] attribute
        for attr in &field.attrs {
            if attr.path().is_ident("rentfree") {
                // Handle both #[rentfree] and #[rentfree(...)]
                let args: RentFreeArgs = match &attr.meta {
                    syn::Meta::Path(_) => {
                        // No parentheses: #[rentfree]
                        RentFreeArgs {
                            address_tree_info: None,
                            output_tree: None,
                        }
                    }
                    syn::Meta::List(_) => {
                        // Has parentheses: #[rentfree(...)]
                        attr.parse_args()?
                    }
                    syn::Meta::NameValue(_) => {
                        return Err(Error::new_spanned(
                            attr,
                            "expected #[rentfree] or #[rentfree(...)]",
                        ));
                    }
                };

                // Use defaults if not specified
                let address_tree_info = args.address_tree_info.unwrap_or_else(|| {
                    syn::parse_quote!(params.create_accounts_proof.address_tree_info)
                });
                let output_tree = args.output_tree.unwrap_or_else(|| {
                    syn::parse_quote!(params.create_accounts_proof.output_state_tree_index)
                });

                // Validate this is an Account type
                let (is_boxed, _inner_type) =
                    extract_account_inner_type(&field.ty).ok_or_else(|| {
                        Error::new_spanned(
                            &field.ty,
                            "#[rentfree] can only be applied to Account<...> fields",
                        )
                    })?;

                rentfree_fields.push(RentFreeField {
                    ident: field_ident.clone(),
                    ty: field.ty.clone(),
                    address_tree_info,
                    output_tree,
                    is_boxed,
                });
                break;
            }

            // Look for #[light_mint(...)] attribute
            if attr.path().is_ident("light_mint") {
                let args: LightMintArgs = attr.parse_args()?;

                // Validate required fields
                let mint_signer = args
                    .mint_signer
                    .ok_or_else(|| Error::new_spanned(attr, "light_mint requires mint_signer"))?;
                let authority = args
                    .authority
                    .ok_or_else(|| Error::new_spanned(attr, "light_mint requires authority"))?;
                let decimals = args
                    .decimals
                    .ok_or_else(|| Error::new_spanned(attr, "light_mint requires decimals"))?;

                // address_tree_info defaults to params.create_accounts_proof.address_tree_info
                let address_tree_info = args.address_tree_info.unwrap_or_else(|| {
                    syn::parse_quote!(params.create_accounts_proof.address_tree_info)
                });

                light_mint_fields.push(LightMintField {
                    field_ident: field_ident.clone(),
                    mint_signer,
                    authority,
                    decimals,
                    address_tree_info,
                    freeze_authority: args.freeze_authority,
                    signer_seeds: args.signer_seeds,
                    rent_payment: args.rent_payment,
                    write_top_up: args.write_top_up,
                });
                break;
            }
        }
    }

    Ok(ParsedCompressibleStruct {
        struct_name,
        generics,
        rentfree_fields,
        light_mint_fields,
        instruction_args,
        fee_payer_field,
        compression_config_field,
        ctoken_config_field,
        ctoken_rent_sponsor_field,
        ctoken_program_field,
        ctoken_cpi_authority_field,
    })
}
