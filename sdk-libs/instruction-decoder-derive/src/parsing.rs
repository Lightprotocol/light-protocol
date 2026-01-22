//! Darling-based attribute parsing for instruction decoder macros.
//!
//! This module provides declarative attribute parsing using the darling crate,
//! replacing manual `parse_nested_meta` implementations with type-safe structs.
//!
//! # Supported Attributes
//!
//! ## Derive macro (`#[derive(InstructionDecoder)]`)
//!
//! Top-level:
//! ```ignore
//! #[instruction_decoder(
//!     program_id = "Base58ProgramId...",
//!     program_name = "My Program",      // optional, defaults to enum name
//!     discriminator_size = 8            // optional: 1, 4, or 8 (default: 8)
//! )]
//! ```
//!
//! Variant-level:
//! ```ignore
//! #[instruction_decoder(
//!     accounts = MyAccounts,            // Accounts struct implementing ACCOUNT_NAMES
//!     params = MyParams,                // Params struct implementing BorshDeserialize + Debug
//!     account_names = ["a", "b", "c"]   // Inline account names (alternative to accounts)
//! )]
//! #[discriminator = 5]                  // Explicit discriminator value (for 1/4 byte modes)
//! ```

use darling::{FromDeriveInput, FromMeta, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, ExprLit, Ident, Lit, Type};

use crate::{
    crate_context::CrateContext,
    utils::{parse_program_id_bytes, pascal_to_display, validate_discriminator_size},
};

/// Default discriminator size (Anchor-style 8 bytes).
fn default_discriminator_size() -> u8 {
    8
}

/// Top-level attributes for `#[derive(InstructionDecoder)]`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(instruction_decoder), supports(enum_any))]
pub struct InstructionDecoderArgs {
    /// The enum identifier
    pub ident: Ident,

    /// Base58-encoded program ID string
    pub program_id: String,

    /// Human-readable program name (defaults to enum name with spaces)
    #[darling(default)]
    pub program_name: Option<String>,

    /// Discriminator size in bytes: 1 (native), 4 (system), or 8 (Anchor)
    #[darling(default = "default_discriminator_size")]
    pub discriminator_size: u8,

    /// Enum data for accessing variants
    pub data: darling::ast::Data<VariantDecoderArgs, ()>,
}

impl InstructionDecoderArgs {
    /// Get the display name for this program.
    pub fn display_name(&self) -> String {
        self.program_name
            .clone()
            .unwrap_or_else(|| pascal_to_display(&self.ident.to_string()))
    }

    /// Parse and validate the program ID, returning a token stream for the byte array.
    pub fn program_id_bytes(&self, span: proc_macro2::Span) -> syn::Result<TokenStream2> {
        parse_program_id_bytes(&self.program_id, span)
    }

    /// Validate all arguments.
    pub fn validate(&self) -> syn::Result<()> {
        validate_discriminator_size(self.discriminator_size, self.ident.span())?;
        // Validate program_id can be parsed (will error at code gen time if invalid)
        let _ = self.program_id_bytes(self.ident.span())?;
        Ok(())
    }

    /// Get variants as a slice.
    pub fn variants(&self) -> &[VariantDecoderArgs] {
        match &self.data {
            darling::ast::Data::Enum(variants) => variants,
            _ => &[],
        }
    }
}

/// Account names specification - either inline strings or a type reference.
#[derive(Debug, Clone)]
pub enum AccountNamesSpec {
    /// Inline list of account name strings
    Inline(Vec<String>),
    /// Type reference with ACCOUNT_NAMES constant (boxed to reduce enum size)
    TypeRef(Box<Type>),
}

/// Wrapper for Type that implements FromMeta by parsing the value as a path.
#[derive(Debug, Clone)]
pub struct TypeWrapper(pub Type);

impl FromMeta for TypeWrapper {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                // Parse the expression as a type (path)
                let ty: Type = match &nv.value {
                    Expr::Path(expr_path) => Type::Path(syn::TypePath {
                        qself: None,
                        path: expr_path.path.clone(),
                    }),
                    _ => {
                        return Err(
                            darling::Error::custom("expected a type path").with_span(&nv.value)
                        );
                    }
                };
                Ok(TypeWrapper(ty))
            }
            _ => Err(darling::Error::custom("expected name = Type").with_span(item)),
        }
    }
}

/// Variant-level attributes for instruction decoder.
#[derive(Debug, FromVariant)]
#[darling(attributes(instruction_decoder))]
pub struct VariantDecoderArgs {
    /// Variant identifier (required by darling, used for error messages)
    #[allow(dead_code)]
    pub ident: Ident,

    /// Variant fields (required by darling)
    #[allow(dead_code)]
    pub fields: darling::ast::Fields<syn::Field>,

    /// Accounts struct type (e.g., `CreateRecord`)
    #[darling(default)]
    pub accounts: Option<TypeWrapper>,

    /// Params struct type for borsh deserialization
    #[darling(default)]
    pub params: Option<TypeWrapper>,

    /// Inline account names (e.g., `["source", "dest"]`)
    #[darling(default)]
    pub account_names: Option<InlineAccountNames>,

    /// Optional pretty formatter function path (e.g., `crate::programs::ctoken::format_transfer2`).
    /// The function must have signature `fn(&ParamsType, &[AccountMeta]) -> String`.
    #[darling(default)]
    pub pretty_formatter: Option<syn::Path>,

    /// Optional function to resolve account names dynamically from parsed params.
    /// The function must have signature `fn(&ParamsType, &[AccountMeta]) -> Vec<String>`.
    /// When specified, this takes precedence over `accounts` and `account_names`.
    #[darling(default)]
    pub account_names_resolver_from_params: Option<syn::Path>,
}

impl VariantDecoderArgs {
    /// Get the account names specification for this variant.
    pub fn account_names_spec(&self) -> Option<AccountNamesSpec> {
        if let Some(ref inline) = self.account_names {
            Some(AccountNamesSpec::Inline(inline.0.clone()))
        } else {
            self.accounts
                .as_ref()
                .map(|wrapper| AccountNamesSpec::TypeRef(Box::new(wrapper.0.clone())))
        }
    }

    /// Get the params type if specified.
    pub fn params_type(&self) -> Option<&Type> {
        self.params.as_ref().map(|wrapper| &wrapper.0)
    }

    /// Generate code to produce account names at runtime.
    ///
    /// If `accounts` type is specified, looks up field names from CrateContext.
    /// If `account_names` inline list is specified, uses those directly.
    ///
    /// Emits compile-time warnings if struct resolution fails.
    pub fn account_names_code(&self, crate_ctx: Option<&CrateContext>) -> TokenStream2 {
        match self.account_names_spec() {
            Some(AccountNamesSpec::Inline(names)) => {
                // Inline names - use directly
                quote! { vec![#(#names.to_string()),*] }
            }
            Some(AccountNamesSpec::TypeRef(ty)) => {
                // Type reference - extract struct name and lookup in CrateContext
                let struct_name = extract_struct_name(&ty);
                let variant_name = &self.ident;

                let Some(ctx) = crate_ctx else {
                    eprintln!(
                        "warning: InstructionDecoder variant '{}': could not parse crate context, \
                         account names for '{}' will be empty",
                        variant_name, struct_name
                    );
                    return quote! { Vec::new() };
                };

                if let Some(field_names) = ctx.get_struct_field_names(&struct_name) {
                    // Found in crate - generate inline names
                    return quote! { vec![#(#field_names.to_string()),*] };
                }

                // Struct not found - emit warning and fallback to empty vec
                eprintln!(
                    "warning: InstructionDecoder variant '{}': struct '{}' not found in crate, \
                     account names will be empty. Ensure the struct is defined in this crate.",
                    variant_name, struct_name
                );
                quote! { Vec::new() }
            }
            None => quote! { Vec::new() },
        }
    }
}

/// Extract the simple struct name from a type path.
///
/// Examples:
/// - `instruction_accounts::CreateTwoMints` -> "CreateTwoMints"
/// - `CreateTwoMints` -> "CreateTwoMints"
/// - `crate::foo::Bar` -> "Bar"
fn extract_struct_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Wrapper for parsing inline account names array.
///
/// Supports: `account_names = ["source", "dest", "authority"]`
#[derive(Debug, Clone, Default)]
pub struct InlineAccountNames(pub Vec<String>);

impl FromMeta for InlineAccountNames {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                // Parse the value as an array expression
                if let Expr::Array(arr) = &nv.value {
                    let names: darling::Result<Vec<String>> = arr
                        .elems
                        .iter()
                        .map(|elem| {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) = elem
                            {
                                Ok(s.value())
                            } else {
                                Err(
                                    darling::Error::custom("account_names must be string literals")
                                        .with_span(elem),
                                )
                            }
                        })
                        .collect();
                    Ok(InlineAccountNames(names?))
                } else {
                    Err(
                        darling::Error::custom("account_names must be an array of string literals")
                            .with_span(&nv.value),
                    )
                }
            }
            _ => Err(
                darling::Error::custom("expected account_names = [\"...\", ...]").with_span(item),
            ),
        }
    }
}

/// Explicit discriminator value - either a u32 integer or an 8-byte array.
#[derive(Debug, Clone)]
pub enum ExplicitDiscriminator {
    /// Integer discriminator (for 1 or 4 byte modes)
    U32(u32),
    /// Array discriminator (for 8 byte mode)
    Array([u8; 8]),
}

/// Parse explicit discriminator from `#[discriminator = N]` or `#[discriminator(a, b, c, ...)]` attribute.
///
/// This is separate from darling parsing because it uses a different attribute name.
/// Supports two formats:
/// - Integer literal: `#[discriminator = 5]`
/// - Array (parenthesized): `#[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]`
pub fn parse_explicit_discriminator(
    variant: &syn::Variant,
) -> syn::Result<Option<ExplicitDiscriminator>> {
    for attr in &variant.attrs {
        if attr.path().is_ident("discriminator") {
            // Try name-value format first: #[discriminator = 5]
            if let Ok(meta) = attr.meta.require_name_value() {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit_int),
                    ..
                }) = &meta.value
                {
                    return Ok(Some(ExplicitDiscriminator::U32(
                        lit_int.base10_parse::<u32>()?,
                    )));
                } else {
                    return Err(syn::Error::new_spanned(
                        &meta.value,
                        "discriminator value must be an integer literal (use #[discriminator(a, b, ...)] for arrays)",
                    ));
                }
            }

            // Try list format: #[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]
            if let Ok(meta) = attr.meta.require_list() {
                let bytes: Result<Vec<u8>, syn::Error> = meta
                    .parse_args_with(
                        syn::punctuated::Punctuated::<syn::LitInt, syn::Token![,]>::parse_terminated,
                    )?
                    .iter()
                    .map(|lit| lit.base10_parse::<u8>())
                    .collect();
                let bytes = bytes?;
                if bytes.len() != 8 {
                    return Err(syn::Error::new_spanned(
                        &meta.tokens,
                        format!(
                            "array discriminator must have exactly 8 bytes, found {}",
                            bytes.len()
                        ),
                    ));
                }
                let array: [u8; 8] = bytes.try_into().unwrap();
                return Ok(Some(ExplicitDiscriminator::Array(array)));
            }

            // Neither format worked
            return Err(syn::Error::new_spanned(
                attr,
                "discriminator must be #[discriminator = N] or #[discriminator(a, b, c, d, e, f, g, h)]",
            ));
        }
    }
    Ok(None)
}

/// Represents either a literal pubkey or a path reference for program ID.
#[derive(Debug, Clone)]
pub enum ProgramIdSource {
    /// Literal base58 string converted to bytes
    Bytes(TokenStream2),
    /// Path reference like `crate::ID` or `ID`
    Path(syn::Path),
}

impl ProgramIdSource {
    /// Generate code for the `program_id()` method.
    pub fn program_id_impl(&self) -> TokenStream2 {
        match self {
            ProgramIdSource::Bytes(bytes) => quote! {
                fn program_id(&self) -> light_instruction_decoder::solana_pubkey::Pubkey {
                    light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(#bytes)
                }
            },
            ProgramIdSource::Path(path) => quote! {
                fn program_id(&self) -> light_instruction_decoder::solana_pubkey::Pubkey {
                    #path
                }
            },
        }
    }
}

/// Arguments for the `#[instruction_decoder]` attribute macro on modules.
#[derive(Debug, Default)]
pub struct ModuleDecoderArgs {
    /// Program ID source (bytes or path)
    pub program_id: Option<ProgramIdSource>,
    /// Human-readable program name
    pub program_name: Option<String>,
}

impl ModuleDecoderArgs {
    /// Parse module decoder arguments from attribute tokens.
    pub fn parse(attr: TokenStream2) -> syn::Result<Self> {
        let mut args = ModuleDecoderArgs::default();

        if attr.is_empty() {
            return Ok(args);
        }

        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("program_id") {
                let value = meta.value()?;
                // Try string literal first
                if let Ok(lit) = value.parse::<syn::LitStr>() {
                    let pubkey_str = lit.value();
                    let bytes = bs58::decode(&pubkey_str)
                        .into_vec()
                        .map_err(|_| meta.error("invalid base58 pubkey"))?;
                    if bytes.len() != 32 {
                        return Err(meta.error("pubkey must be 32 bytes"));
                    }
                    args.program_id = Some(ProgramIdSource::Bytes(quote! { [#(#bytes),*] }));
                } else {
                    // Parse as path reference
                    let path: syn::Path = value.parse()?;
                    args.program_id = Some(ProgramIdSource::Path(path));
                }
                Ok(())
            } else if meta.path.is_ident("program_name") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                args.program_name = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("unknown attribute"))
            }
        });

        syn::parse::Parser::parse2(parser, attr)?;
        Ok(args)
    }

    /// Try to find program ID from `declare_id!` macro in module content.
    pub fn find_declare_id(&mut self, module: &syn::ItemMod) -> syn::Result<()> {
        if self.program_id.is_some() {
            return Ok(());
        }

        if let Some(ref content) = module.content {
            for item in &content.1 {
                if let syn::Item::Macro(macro_item) = item {
                    if macro_item.mac.path.is_ident("declare_id") {
                        let tokens = &macro_item.mac.tokens;
                        let lit: syn::LitStr = syn::parse2(tokens.clone())?;
                        let pubkey_str = lit.value();
                        let bytes = bs58::decode(&pubkey_str)
                            .into_vec()
                            .map_err(|_| syn::Error::new_spanned(&lit, "invalid base58 pubkey"))?;
                        if bytes.len() != 32 {
                            return Err(syn::Error::new_spanned(&lit, "pubkey must be 32 bytes"));
                        }
                        self.program_id = Some(ProgramIdSource::Bytes(quote! { [#(#bytes),*] }));
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    /// Get program ID source, defaulting to `crate::ID` if not specified.
    pub fn program_id_source(&self) -> ProgramIdSource {
        self.program_id
            .clone()
            .unwrap_or_else(|| ProgramIdSource::Path(syn::parse_quote!(crate::ID)))
    }

    /// Get program name, defaulting to module name with spaces.
    pub fn program_name(&self, module_name: &str) -> String {
        self.program_name.clone().unwrap_or_else(|| {
            let pascal = crate::utils::to_pascal_case(module_name);
            pascal_to_display(&pascal)
        })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_inline_account_names_parsing() {
        let meta: syn::Meta = parse_quote!(account_names = ["source", "dest", "authority"]);
        let result = InlineAccountNames::from_meta(&meta).unwrap();
        assert_eq!(result.0, vec!["source", "dest", "authority"]);
    }

    #[test]
    fn test_variant_args_with_accounts_type() {
        let variant: syn::Variant = parse_quote! {
            #[instruction_decoder(accounts = CreateRecord)]
            CreateRecord
        };
        let args = VariantDecoderArgs::from_variant(&variant).unwrap();
        assert!(args.accounts.is_some());
        assert!(args.params.is_none());
    }

    #[test]
    fn test_variant_args_with_inline_names() {
        let variant: syn::Variant = parse_quote! {
            #[instruction_decoder(account_names = ["source", "dest"])]
            Transfer
        };
        let args = VariantDecoderArgs::from_variant(&variant).unwrap();
        assert!(args.account_names.is_some());
        let names = args.account_names.unwrap();
        assert_eq!(names.0, vec!["source", "dest"]);
    }

    #[test]
    fn test_parse_explicit_discriminator_u32() {
        let variant: syn::Variant = parse_quote! {
            #[discriminator = 5]
            Transfer
        };
        let disc = parse_explicit_discriminator(&variant).unwrap();
        assert!(matches!(disc, Some(ExplicitDiscriminator::U32(5))));
    }

    #[test]
    fn test_parse_explicit_discriminator_array() {
        let variant: syn::Variant = parse_quote! {
            #[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]
            Invoke
        };
        let disc = parse_explicit_discriminator(&variant).unwrap();
        assert!(matches!(
            disc,
            Some(ExplicitDiscriminator::Array([
                26, 16, 169, 7, 21, 202, 242, 25
            ]))
        ));
    }

    #[test]
    fn test_parse_explicit_discriminator_array_wrong_length() {
        let variant: syn::Variant = parse_quote! {
            #[discriminator(1, 2, 3, 4)]
            Transfer
        };
        let result = parse_explicit_discriminator(&variant);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("array discriminator must have exactly 8 bytes"));
    }

    #[test]
    fn test_parse_explicit_discriminator_none() {
        let variant: syn::Variant = parse_quote! {
            Transfer
        };
        let disc = parse_explicit_discriminator(&variant).unwrap();
        assert!(disc.is_none());
    }

    #[test]
    fn test_extract_struct_name_simple() {
        let ty: syn::Type = parse_quote!(CreateTwoMints);
        assert_eq!(extract_struct_name(&ty), "CreateTwoMints");
    }

    #[test]
    fn test_extract_struct_name_qualified() {
        let ty: syn::Type = parse_quote!(instruction_accounts::CreateTwoMints);
        assert_eq!(extract_struct_name(&ty), "CreateTwoMints");
    }

    #[test]
    fn test_extract_struct_name_crate_path() {
        let ty: syn::Type = parse_quote!(crate::foo::bar::MyStruct);
        assert_eq!(extract_struct_name(&ty), "MyStruct");
    }
}
