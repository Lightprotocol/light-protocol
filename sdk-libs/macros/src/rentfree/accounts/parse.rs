//! Parsing logic for #[rentfree(...)] attributes using darling.

use darling::FromMeta;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Error, Expr, Ident, Token, Type,
};

// Import shared types from seed_extraction module
pub(super) use crate::rentfree::traits::seed_extraction::extract_account_inner_type;

// Import LightMintField and parsing from light_mint module
use super::light_mint::{parse_light_mint_attr, LightMintField};

// ============================================================================
// darling support for parsing Expr from attributes
// ============================================================================

/// Wrapper for syn::Expr that implements darling's FromMeta trait.
/// Enables darling to parse arbitrary expressions in attributes.
#[derive(Clone)]
struct MetaExpr(Expr);

impl FromMeta for MetaExpr {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        Ok(MetaExpr(expr.clone()))
    }
}

impl From<MetaExpr> for Expr {
    fn from(meta: MetaExpr) -> Expr {
        meta.0
    }
}

/// Parsed representation of a struct with rentfree and light_mint fields.
pub(super) struct ParsedRentFreeStruct {
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
    /// The inner type T from Account<'info, T> or Box<Account<'info, T>>
    pub inner_type: Ident,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
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

fn rentfree_args_default() -> darling::Result<RentFreeArgs> {
    Ok(RentFreeArgs::default())
}

/// Arguments inside #[rentfree(...)] parsed by darling.
///
/// Supports both `#[rentfree]` (word format) and `#[rentfree(...)]` (list format).
/// All fields default to None if not specified.
#[derive(FromMeta, Default)]
#[darling(default, from_word = rentfree_args_default)]
struct RentFreeArgs {
    /// Address tree info expression
    address_tree_info: Option<MetaExpr>,
    /// Output tree index expression
    output_tree: Option<MetaExpr>,
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
pub(super) fn parse_rentfree_struct(
    input: &DeriveInput,
) -> Result<ParsedRentFreeStruct, Error> {
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
        match field_name.as_str() {
            "fee_payer" | "payer" | "creator" => {
                fee_payer_field = Some(field_ident.clone());
            }
            "compression_config" => {
                compression_config_field = Some(field_ident.clone());
            }
            "ctoken_compressible_config" | "ctoken_config" | "light_token_config_account" => {
                ctoken_config_field = Some(field_ident.clone());
            }
            "ctoken_rent_sponsor" | "light_token_rent_sponsor" => {
                ctoken_rent_sponsor_field = Some(field_ident.clone());
            }
            "ctoken_program" | "light_token_program" => {
                ctoken_program_field = Some(field_ident.clone());
            }
            "ctoken_cpi_authority"
            | "light_token_program_cpi_authority"
            | "compress_token_program_cpi_authority" => {
                ctoken_cpi_authority_field = Some(field_ident.clone());
            }
            _ => {}
        }

        // Track if this field already has a compression attribute
        let mut has_compression_attr = false;

        // Check for #[light_mint(...)] attribute first (delegated to light_mint module)
        if let Some(mint_field) = parse_light_mint_attr(field, &field_ident)? {
            has_compression_attr = true;
            light_mint_fields.push(mint_field);
        }

        // Look for #[rentfree] or #[rentfree(...)] attribute
        for attr in &field.attrs {
            if attr.path().is_ident("rentfree") {
                // Check for duplicate compression attributes on same field
                if has_compression_attr {
                    return Err(Error::new_spanned(
                        attr,
                        "Field already has a compression attribute (#[rentfree] or #[light_mint]). \
                         Only one is allowed per field.",
                    ));
                }

                // Use darling to parse the attribute arguments
                // Handles both #[rentfree] and #[rentfree(...)]
                let args = RentFreeArgs::from_meta(&attr.meta)
                    .map_err(|e| Error::new_spanned(attr, e.to_string()))?;

                // Use defaults if not specified
                let address_tree_info = args
                    .address_tree_info
                    .map(Into::into)
                    .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.address_tree_info));
                let output_tree = args
                    .output_tree
                    .map(Into::into)
                    .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.output_state_tree_index));

                // Validate this is an Account type (or Box<Account>)
                let (is_boxed, inner_type) =
                    extract_account_inner_type(&field.ty).ok_or_else(|| {
                        Error::new_spanned(
                            &field.ty,
                            "#[rentfree] can only be applied to Account<...> or Box<Account<...>> fields. \
                             Nested Box<Box<...>> is not supported.",
                        )
                    })?;

                rentfree_fields.push(RentFreeField {
                    ident: field_ident.clone(),
                    inner_type,
                    address_tree_info,
                    output_tree,
                    is_boxed,
                });
                break;
            }

            // TODO(diff-pr): Add parsing for #[rentfree_token(...)] attribute for token accounts and ATAs.
            // Would need RentFreeTokenField struct with: field_ident, authority_seeds, mint field ref.
        }
    }

    // Validation: #[rentfree] and #[light_mint] require #[instruction] attribute
    if (!rentfree_fields.is_empty() || !light_mint_fields.is_empty())
        && instruction_args.is_none()
    {
        return Err(Error::new_spanned(
            input,
            "#[rentfree] and #[light_mint] fields require #[instruction(params: YourParamsType)] \
             attribute on the struct",
        ));
    }

    Ok(ParsedRentFreeStruct {
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
