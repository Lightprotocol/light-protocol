//! Parsing logic for #[light_account(...)] attributes.
//!
//! This module handles struct-level parsing and field classification.
//! The unified #[light_account] attribute parsing is in `light_account.rs`.

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Error, Expr, Ident, Token, Type,
};

// Import unified parsing from light_account module
use super::light_account::{
    parse_light_account_attr, AtaField, LightAccountField, TokenAccountField,
};
// Import LightMintField from mint module (for type export)
pub(super) use super::mint::LightMintField;

// ============================================================================
// Infrastructure Field Classification
// ============================================================================

/// Classification of infrastructure fields by naming convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InfraFieldType {
    FeePayer,
    CompressionConfig,
    LightTokenConfig,
    LightTokenRentSponsor,
    LightTokenProgram,
    LightTokenCpiAuthority,
}

impl InfraFieldType {
    /// Returns the accepted field names for this infrastructure type.
    pub fn accepted_names(&self) -> &'static [&'static str] {
        match self {
            InfraFieldType::FeePayer => &["fee_payer", "payer", "creator"],
            InfraFieldType::CompressionConfig => &["compression_config"],
            InfraFieldType::LightTokenConfig => &["light_token_compressible_config"],
            InfraFieldType::LightTokenRentSponsor => &["light_token_rent_sponsor", "rent_sponsor"],
            InfraFieldType::LightTokenProgram => &["light_token_program"],
            InfraFieldType::LightTokenCpiAuthority => &["light_token_cpi_authority"],
        }
    }

    /// Human-readable description for error messages.
    pub fn description(&self) -> &'static str {
        match self {
            InfraFieldType::FeePayer => "fee payer (transaction signer)",
            InfraFieldType::CompressionConfig => "compression config",
            InfraFieldType::LightTokenConfig => "light token compressible config",
            InfraFieldType::LightTokenRentSponsor => "light token rent sponsor",
            InfraFieldType::LightTokenProgram => "light token program",
            InfraFieldType::LightTokenCpiAuthority => "light token CPI authority",
        }
    }
}

/// Classifier for infrastructure fields by naming convention.
pub(super) struct InfraFieldClassifier;

impl InfraFieldClassifier {
    /// Classify a field name into its infrastructure type, if any.
    #[inline]
    pub fn classify(name: &str) -> Option<InfraFieldType> {
        match name {
            "fee_payer" | "payer" | "creator" => Some(InfraFieldType::FeePayer),
            "compression_config" => Some(InfraFieldType::CompressionConfig),
            "light_token_compressible_config" => Some(InfraFieldType::LightTokenConfig),
            "light_token_rent_sponsor" | "rent_sponsor" => {
                Some(InfraFieldType::LightTokenRentSponsor)
            }
            "light_token_program" => Some(InfraFieldType::LightTokenProgram),
            "light_token_cpi_authority" => Some(InfraFieldType::LightTokenCpiAuthority),
            _ => None,
        }
    }
}

/// Collected infrastructure field identifiers.
#[derive(Default)]
pub(super) struct InfraFields {
    pub fee_payer: Option<Ident>,
    pub compression_config: Option<Ident>,
    pub light_token_config: Option<Ident>,
    pub light_token_rent_sponsor: Option<Ident>,
    pub light_token_program: Option<Ident>,
    pub light_token_cpi_authority: Option<Ident>,
}

impl InfraFields {
    /// Set an infrastructure field by type.
    /// Returns an error if the field is already set (duplicate detection).
    pub fn set(&mut self, field_type: InfraFieldType, ident: Ident) -> Result<(), Error> {
        match field_type {
            InfraFieldType::FeePayer => {
                if self.fee_payer.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: fee_payer",
                    ));
                }
                self.fee_payer = Some(ident);
            }
            InfraFieldType::CompressionConfig => {
                if self.compression_config.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: compression_config",
                    ));
                }
                self.compression_config = Some(ident);
            }
            InfraFieldType::LightTokenConfig => {
                if self.light_token_config.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: light_token_config",
                    ));
                }
                self.light_token_config = Some(ident);
            }
            InfraFieldType::LightTokenRentSponsor => {
                if self.light_token_rent_sponsor.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: light_token_rent_sponsor",
                    ));
                }
                self.light_token_rent_sponsor = Some(ident);
            }
            InfraFieldType::LightTokenProgram => {
                if self.light_token_program.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: light_token_program",
                    ));
                }
                self.light_token_program = Some(ident);
            }
            InfraFieldType::LightTokenCpiAuthority => {
                if self.light_token_cpi_authority.is_some() {
                    return Err(Error::new_spanned(
                        &ident,
                        "duplicate infrastructure field: light_token_cpi_authority",
                    ));
                }
                self.light_token_cpi_authority = Some(ident);
            }
        }
        Ok(())
    }
}

/// Parsed representation of a struct with rentfree and light_mint fields.
pub(super) struct ParsedLightAccountsStruct {
    pub struct_name: Ident,
    pub generics: syn::Generics,
    pub rentfree_fields: Vec<ParsedPdaField>,
    pub light_mint_fields: Vec<LightMintField>,
    pub token_account_fields: Vec<TokenAccountField>,
    pub ata_fields: Vec<AtaField>,
    pub instruction_args: Option<Vec<InstructionArg>>,
    /// Infrastructure fields detected by naming convention.
    pub infra_fields: InfraFields,
    /// If CreateAccountsProof type is passed as a direct instruction arg, stores arg name.
    /// Matched by TYPE, not by name - allows any argument name (e.g., `proof`, `create_proof`).
    pub direct_proof_arg: Option<Ident>,
}

/// A field marked with #[light_account(init)]
#[allow(dead_code)] // is_zero_copy is read via From<PdaField> conversion in program module
pub(super) struct ParsedPdaField {
    pub ident: Ident,
    /// The inner type T from Account<'info, T> or Box<Account<'info, T>>
    /// Preserves the full type path (e.g., crate::state::UserRecord).
    pub inner_type: Type,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
    /// True if the field uses zero-copy serialization (AccountLoader)
    pub is_zero_copy: bool,
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

/// Check if a type is `CreateAccountsProof` (match last path segment).
/// Supports both simple `CreateAccountsProof` and fully qualified paths like
/// `light_sdk::CreateAccountsProof`.
fn is_create_accounts_proof_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "CreateAccountsProof";
        }
    }
    false
}

/// Find if any instruction argument has type `CreateAccountsProof`.
/// Returns the argument's name (Ident) if found.
///
/// Returns an error if multiple `CreateAccountsProof` arguments are found,
/// as this would make proof access ambiguous.
fn find_direct_proof_arg(
    instruction_args: &Option<Vec<InstructionArg>>,
) -> Result<Option<Ident>, Error> {
    let Some(args) = instruction_args.as_ref() else {
        return Ok(None);
    };

    let proof_args: Vec<_> = args
        .iter()
        .filter(|arg| is_create_accounts_proof_type(&arg.ty))
        .collect();

    match proof_args.len() {
        0 => Ok(None),
        1 => Ok(Some(proof_args[0].name.clone())),
        _ => {
            let names: Vec<_> = proof_args.iter().map(|a| a.name.to_string()).collect();
            Err(Error::new_spanned(
                &proof_args[1].name,
                format!(
                    "Multiple CreateAccountsProof arguments found: [{}]. \
                     Only one CreateAccountsProof argument is allowed per instruction.",
                    names.join(", ")
                ),
            ))
        }
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

/// Parse a struct to extract light_account fields (PDAs and mints).
pub(super) fn parse_light_accounts_struct(
    input: &DeriveInput,
) -> Result<ParsedLightAccountsStruct, Error> {
    let struct_name = input.ident.clone();
    let generics = input.generics.clone();

    let instruction_args = parse_instruction_attr(&input.attrs)?;

    // Check if CreateAccountsProof is passed as a direct instruction argument
    // (compute this early so we can use it for field parsing defaults)
    let direct_proof_arg = find_direct_proof_arg(&instruction_args)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new_spanned(input, "expected named fields")),
        },
        _ => return Err(Error::new_spanned(input, "expected struct")),
    };

    let mut rentfree_fields = Vec::new();
    let mut light_mint_fields = Vec::new();
    let mut token_account_fields = Vec::new();
    let mut ata_fields = Vec::new();
    let mut infra_fields = InfraFields::default();

    for field in fields {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "expected named field with identifier"))?;
        let field_name = field_ident.to_string();

        // Track infrastructure fields by naming convention using the classifier.
        // See InfraFieldClassifier for supported field names.
        if let Some(field_type) = InfraFieldClassifier::classify(&field_name) {
            infra_fields.set(field_type, field_ident.clone())?;
        }

        // Check for #[light_account(...)] - the unified syntax
        if let Some(light_account_field) =
            parse_light_account_attr(field, &field_ident, &direct_proof_arg)?
        {
            match light_account_field {
                LightAccountField::Pda(pda) => rentfree_fields.push((*pda).into()),
                LightAccountField::Mint(mint) => light_mint_fields.push(*mint),
                LightAccountField::TokenAccount(token) => token_account_fields.push(*token),
                LightAccountField::AssociatedToken(ata) => ata_fields.push(*ata),
            }
            continue; // Field processed, move to next
        }
    }

    // Validation: #[light_account] fields require #[instruction] attribute
    let has_light_account_fields = !rentfree_fields.is_empty()
        || !light_mint_fields.is_empty()
        || !token_account_fields.is_empty()
        || !ata_fields.is_empty();
    if has_light_account_fields && instruction_args.is_none() {
        return Err(Error::new_spanned(
            input,
            "#[light_account] fields require #[instruction(params: YourParamsType)] \
             attribute on the struct",
        ));
    }

    Ok(ParsedLightAccountsStruct {
        struct_name,
        generics,
        rentfree_fields,
        light_mint_fields,
        token_account_fields,
        ata_fields,
        instruction_args,
        infra_fields,
        direct_proof_arg,
    })
}
