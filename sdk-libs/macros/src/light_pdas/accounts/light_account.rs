//! Unified #[light_account(...)] attribute parsing.
//!
//! This module provides a single unified syntax for all Light Protocol account types:
//! - `#[light_account(init)]` - PDAs (replaces `#[light_account(init)]`)
//! - `#[light_account(init, mint, ...)]` - Compressed mints (replaces `#[light_mint]`)
//! - `#[light_account(token, ...)]` - Token accounts for compression (handled by light_program)
//!
//! Note: Token fields are NOT processed here - they're handled by seed_extraction.rs
//! in the light_program macro. This parser returns None for token fields.

use syn::{
    parse::{Parse, ParseStream},
    Error, Expr, Field, Ident, Token, Type,
};

use super::mint::LightMintField;
pub(super) use crate::light_pdas::account::seed_extraction::extract_account_inner_type;

// ============================================================================
// Account Type Classification
// ============================================================================

/// Account type specifier parsed from the attribute.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LightAccountType {
    #[default]
    Pda, // Default (no type specifier) - for PDAs
    Mint, // `mint` keyword - for compressed mints
          // Future:
          // TokenAccount, // `token_account` keyword
          // Ata,          // `ata` keyword
}

// ============================================================================
// Unified Parsed Result
// ============================================================================

/// Unified representation of a #[light_account(...)] field.
pub enum LightAccountField {
    Pda(Box<PdaField>),
    Mint(Box<LightMintField>),
}

/// A field marked with #[light_account(init)] (PDA).
pub struct PdaField {
    pub ident: Ident,
    /// The inner type T from Account<'info, T> or Box<Account<'info, T>>
    pub inner_type: Type,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
}

// ============================================================================
// Custom Parser for #[light_account(init, [mint,] key = value, ...)]
// ============================================================================

/// Key-value pair in the attribute arguments.
struct KeyValue {
    key: Ident,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(Self { key, value })
    }
}

/// Parsed arguments from #[light_account(init, [mint,] ...)].
struct LightAccountArgs {
    /// True if `init` keyword is present (required for PDA/Mint).
    has_init: bool,
    /// True if `token` keyword is present (marks token fields - skip in LightAccounts derive).
    is_token: bool,
    /// The account type (Pda, Mint, etc.).
    account_type: LightAccountType,
    /// Key-value pairs for additional arguments.
    key_values: Vec<KeyValue>,
}

impl Parse for LightAccountArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First token must be `init` or `token`
        let first: Ident = input.parse()?;

        // If first argument is `token`, this is a token field handled by light_program
        // Consume all remaining tokens and return
        if first == "token" {
            // Consume remaining tokens (e.g., ", authority = [...]")
            while !input.is_empty() {
                let _: proc_macro2::TokenTree = input.parse()?;
            }
            return Ok(Self {
                has_init: false,
                is_token: true,
                account_type: LightAccountType::Pda, // not used for token
                key_values: Vec::new(),
            });
        }

        if first != "init" {
            return Err(Error::new_spanned(
                &first,
                "First argument to #[light_account] must be `init` or `token`",
            ));
        }

        let mut account_type = LightAccountType::Pda;
        let mut key_values = Vec::new();

        // Parse remaining tokens
        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            // Check if this is a type keyword (mint, token_account, ata)
            if input.peek(Ident) {
                let lookahead = input.fork();
                let ident: Ident = lookahead.parse()?;

                // Check for type keywords
                if ident == "mint" && !lookahead.peek(Token![=]) {
                    input.parse::<Ident>()?; // consume it
                    account_type = LightAccountType::Mint;
                    continue;
                }
                // Future: token_account, ata keywords
            }

            // Otherwise it's a key-value pair
            if !input.is_empty() {
                let kv: KeyValue = input.parse()?;
                key_values.push(kv);
            }
        }

        Ok(Self {
            has_init: true,
            is_token: false,
            account_type,
            key_values,
        })
    }
}

// ============================================================================
// Main Parsing Function
// ============================================================================

/// Parse #[light_account(...)] attribute from a field.
/// Returns None if no light_account attribute or if it's a token field (handled elsewhere).
/// Returns Some(LightAccountField) for PDA or Mint fields.
pub(super) fn parse_light_account_attr(
    field: &Field,
    field_ident: &Ident,
) -> Result<Option<LightAccountField>, syn::Error> {
    for attr in &field.attrs {
        if attr.path().is_ident("light_account") {
            let args: LightAccountArgs = attr.parse_args()?;

            // Token fields are handled by light_program macro (seed_extraction.rs)
            // Return None so LightAccounts derive skips them
            if args.is_token {
                return Ok(None);
            }

            if !args.has_init {
                return Err(Error::new_spanned(
                    attr,
                    "#[light_account] requires `init` as the first argument (or use `token` for token accounts)",
                ));
            }

            return match args.account_type {
                LightAccountType::Pda => Ok(Some(LightAccountField::Pda(Box::new(
                    build_pda_field(field, field_ident, &args.key_values)?,
                )))),
                LightAccountType::Mint => Ok(Some(LightAccountField::Mint(Box::new(
                    build_mint_field(field_ident, &args.key_values, attr)?,
                )))),
            };
        }
    }
    Ok(None)
}

/// Build a PdaField from parsed key-value pairs.
fn build_pda_field(
    field: &Field,
    field_ident: &Ident,
    key_values: &[KeyValue],
) -> Result<PdaField, syn::Error> {
    let mut address_tree_info: Option<Expr> = None;
    let mut output_tree: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "address_tree_info" => address_tree_info = Some(kv.value.clone()),
            "output_tree" => output_tree = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown argument `{other}` for PDA. Expected: address_tree_info, output_tree"
                    ),
                ));
            }
        }
    }

    // Use defaults if not specified
    let address_tree_info = address_tree_info
        .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.address_tree_info));
    let output_tree = output_tree
        .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.output_state_tree_index));

    // Validate this is an Account type (or Box<Account>)
    let (is_boxed, inner_type) = extract_account_inner_type(&field.ty).ok_or_else(|| {
        Error::new_spanned(
            &field.ty,
            "#[light_account(init)] can only be applied to Account<...> or Box<Account<...>> fields. \
             Nested Box<Box<...>> is not supported.",
        )
    })?;

    Ok(PdaField {
        ident: field_ident.clone(),
        inner_type,
        address_tree_info,
        output_tree,
        is_boxed,
    })
}

/// Build a LightMintField from parsed key-value pairs.
fn build_mint_field(
    field_ident: &Ident,
    key_values: &[KeyValue],
    attr: &syn::Attribute,
) -> Result<LightMintField, syn::Error> {
    // Required fields
    let mut mint_signer: Option<Expr> = None;
    let mut authority: Option<Expr> = None;
    let mut decimals: Option<Expr> = None;
    let mut mint_seeds: Option<Expr> = None;

    // Optional fields
    let mut address_tree_info: Option<Expr> = None;
    let mut freeze_authority: Option<Ident> = None;
    let mut authority_seeds: Option<Expr> = None;
    let mut rent_payment: Option<Expr> = None;
    let mut write_top_up: Option<Expr> = None;

    // Metadata fields
    let mut name: Option<Expr> = None;
    let mut symbol: Option<Expr> = None;
    let mut uri: Option<Expr> = None;
    let mut update_authority: Option<Ident> = None;
    let mut additional_metadata: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "mint_signer" => mint_signer = Some(kv.value.clone()),
            "authority" => authority = Some(kv.value.clone()),
            "decimals" => decimals = Some(kv.value.clone()),
            "mint_seeds" => mint_seeds = Some(kv.value.clone()),
            "address_tree_info" => address_tree_info = Some(kv.value.clone()),
            "freeze_authority" => {
                freeze_authority = Some(expr_to_ident(&kv.value, "freeze_authority")?);
            }
            "authority_seeds" => authority_seeds = Some(kv.value.clone()),
            "rent_payment" => rent_payment = Some(kv.value.clone()),
            "write_top_up" => write_top_up = Some(kv.value.clone()),
            "name" => name = Some(kv.value.clone()),
            "symbol" => symbol = Some(kv.value.clone()),
            "uri" => uri = Some(kv.value.clone()),
            "update_authority" => {
                update_authority = Some(expr_to_ident(&kv.value, "update_authority")?);
            }
            "additional_metadata" => additional_metadata = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!("Unknown argument `{other}` for mint"),
                ));
            }
        }
    }

    // Validate required fields
    let mint_signer = mint_signer.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint_signer`",
        )
    })?;
    let authority = authority.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `authority`",
        )
    })?;
    let decimals = decimals.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `decimals`",
        )
    })?;
    let mint_seeds = mint_seeds.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint_seeds`",
        )
    })?;

    // Validate metadata fields (all-or-nothing rule)
    validate_metadata_fields(
        &name,
        &symbol,
        &uri,
        &update_authority,
        &additional_metadata,
        attr,
    )?;

    // address_tree_info defaults to params.create_accounts_proof.address_tree_info
    let address_tree_info = address_tree_info
        .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.address_tree_info));

    Ok(LightMintField {
        field_ident: field_ident.clone(),
        mint_signer,
        authority,
        decimals,
        address_tree_info,
        freeze_authority,
        mint_seeds,
        authority_seeds,
        rent_payment,
        write_top_up,
        name,
        symbol,
        uri,
        update_authority,
        additional_metadata,
    })
}

/// Convert an expression to an identifier (for field references).
fn expr_to_ident(expr: &Expr, field_name: &str) -> Result<Ident, syn::Error> {
    match expr {
        Expr::Path(path) => path.path.get_ident().cloned().ok_or_else(|| {
            Error::new_spanned(expr, format!("`{field_name}` must be a simple identifier"))
        }),
        _ => Err(Error::new_spanned(
            expr,
            format!("`{field_name}` must be a simple identifier"),
        )),
    }
}

/// Validates TokenMetadata field requirements.
///
/// Rules:
/// 1. `name`, `symbol`, `uri` must all be defined together or none
/// 2. `update_authority` and `additional_metadata` require `name`, `symbol`, `uri`
fn validate_metadata_fields(
    name: &Option<Expr>,
    symbol: &Option<Expr>,
    uri: &Option<Expr>,
    update_authority: &Option<Ident>,
    additional_metadata: &Option<Expr>,
    attr: &syn::Attribute,
) -> Result<(), syn::Error> {
    let has_name = name.is_some();
    let has_symbol = symbol.is_some();
    let has_uri = uri.is_some();
    let has_update_authority = update_authority.is_some();
    let has_additional_metadata = additional_metadata.is_some();

    let core_metadata_count = [has_name, has_symbol, has_uri]
        .iter()
        .filter(|&&x| x)
        .count();

    // Rule 1: name, symbol, uri must all be defined together or none
    if core_metadata_count > 0 && core_metadata_count < 3 {
        return Err(Error::new_spanned(
            attr,
            "TokenMetadata requires all of `name`, `symbol`, and `uri` to be specified together",
        ));
    }

    // Rule 2: update_authority and additional_metadata require name, symbol, uri
    if (has_update_authority || has_additional_metadata) && core_metadata_count == 0 {
        return Err(Error::new_spanned(
            attr,
            "`update_authority` and `additional_metadata` require `name`, `symbol`, and `uri` to also be specified",
        ));
    }

    Ok(())
}

// ============================================================================
// Conversion to existing types (for compatibility with existing code gen)
// ============================================================================

/// Convert PdaField to ParsedPdaField (used by existing codegen).
impl From<PdaField> for super::parse::ParsedPdaField {
    fn from(pda: PdaField) -> Self {
        Self {
            ident: pda.ident,
            inner_type: pda.inner_type,
            address_tree_info: pda.address_tree_info,
            output_tree: pda.output_tree,
            is_boxed: pda.is_boxed,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_light_account_pda_bare() {
        let field: syn::Field = parse_quote! {
            #[light_account(init)]
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Pda(pda) => {
                assert_eq!(pda.ident.to_string(), "record");
                assert!(!pda.is_boxed);
            }
            _ => panic!("Expected PDA field"),
        }
    }

    #[test]
    fn test_parse_light_account_pda_with_options() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, address_tree_info = custom_tree, output_tree = custom_output)]
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Pda(_) => {}
            _ => panic!("Expected PDA field"),
        }
    }

    #[test]
    fn test_parse_light_account_mint() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"]
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_light_account_mint_with_metadata() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"],
                name = params.name.clone(),
                symbol = params.symbol.clone(),
                uri = params.uri.clone()
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert!(mint.name.is_some());
                assert!(mint.symbol.is_some());
                assert!(mint.uri.is_some());
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_light_account_missing_init() {
        let field: syn::Field = parse_quote! {
            #[light_account(mint, decimals = 9)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_mint_missing_required() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint, decimals = 9)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_partial_metadata_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"],
                name = params.name.clone()
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_light_account_attr_returns_none() {
        let field: syn::Field = parse_quote! {
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
