//! Struct-level validation for LightAccounts derive macro.
//!
//! This module contains validation logic that requires only boolean flags
//! and the struct name for error spans. Attribute-level and field-level
//! validations remain in their respective modules.
//!
//! # Validation Rules
//!
//! 1. **Account count limit** - Total compression fields (PDAs + mints + tokens + ATAs)
//!    must not exceed 255 (u8 index limit)
//!
//! 2. **Fee payer required** - When any `#[light_account]` fields exist, a `fee_payer`
//!    field is required
//!
//! 3. **PDA compression config** - When PDAs exist, `compression_config` field is required
//!
//! 4. **PDA rent sponsor** - When PDAs exist, `pda_rent_sponsor` field is required
//!
//! 5. **Light token config** - When mints, tokens, or ATAs exist,
//!    `light_token_config` field is required
//!
//! 6. **Light token rent sponsor** - When mints, tokens, or ATAs exist,
//!    `light_token_rent_sponsor` field is required
//!
//! 7. **Light token CPI authority** - When mints exist,
//!    `light_token_cpi_authority` field is required
//!
//! 8. **CreateAccountsProof availability** - When PDAs or mints exist,
//!    `CreateAccountsProof` must be available via either:
//!    - Direct instruction argument: `#[instruction(proof: CreateAccountsProof)]`
//!    - Nested in params struct: `#[instruction(params: MyParams)]` where `MyParams`
//!      has `create_accounts_proof` field
//!
//! 9. **Light account fields required** - When `#[instruction]` is present,
//!    at least one `#[light_account]` field must exist. `#[derive(LightAccounts)]`
//!    is only needed for instructions that create light accounts.

use super::parse::InfraFieldType;

/// Context for struct-level validation.
///
/// Contains only the information needed to perform struct-level validation:
/// - Boolean flags indicating presence of various account types
/// - Boolean flags indicating presence of infrastructure fields
/// - The struct name for error spans
pub(super) struct ValidationContext<'a> {
    pub struct_name: &'a syn::Ident,
    pub has_pdas: bool,
    pub has_mints: bool,
    /// Any token accounts (init or mark-only)
    pub has_tokens: bool,
    /// Token accounts with init (requires infrastructure)
    pub has_tokens_with_init: bool,
    /// Any ATAs (init or mark-only)
    pub has_atas: bool,
    /// ATAs with init (requires infrastructure)
    pub has_atas_with_init: bool,
    pub has_fee_payer: bool,
    pub has_compression_config: bool,
    pub has_pda_rent_sponsor: bool,
    pub has_light_token_config: bool,
    pub has_light_token_rent_sponsor: bool,
    pub has_light_token_cpi_authority: bool,
    pub has_instruction_args: bool,
    pub has_direct_proof_arg: bool,
    pub total_account_count: usize,
}

/// Perform all struct-level validations.
pub(super) fn validate_struct(ctx: &ValidationContext) -> Result<(), syn::Error> {
    validate_account_count(ctx)?;
    validate_light_account_fields_required(ctx)?;
    validate_infra_fields(ctx)?;
    validate_proof_availability(ctx)?;
    Ok(())
}

/// Validate that the total account count does not exceed 255.
fn validate_account_count(ctx: &ValidationContext) -> Result<(), syn::Error> {
    if ctx.total_account_count > 255 {
        // For the detailed error message, we need to reconstruct counts
        // This is slightly imprecise (we only have total) but acceptable
        // since 255+ accounts is extremely rare
        return Err(syn::Error::new_spanned(
            ctx.struct_name,
            format!(
                "Too many compression fields ({} total, maximum 255). \
                 Light Protocol uses u8 for account indices.",
                ctx.total_account_count
            ),
        ));
    }
    Ok(())
}

/// Validate that `#[light_account]` fields exist when `#[instruction]` is present.
///
/// `#[derive(LightAccounts)]` is only needed for instructions that create light accounts
/// (rent-free PDAs, mints, token accounts, or ATAs). If there's an `#[instruction]`
/// attribute but no `#[light_account(init, ...)]` fields, the derive macro is unnecessary.
///
/// Note: All `#[light_account]` fields require `init` keyword.
fn validate_light_account_fields_required(ctx: &ValidationContext) -> Result<(), syn::Error> {
    let has_light_account_fields = ctx.has_pdas || ctx.has_mints || ctx.has_tokens || ctx.has_atas;

    if ctx.has_instruction_args && !has_light_account_fields {
        return Err(syn::Error::new_spanned(
            ctx.struct_name,
            "#[derive(LightAccounts)] with #[instruction(...)] requires at least one \
             #[light_account] field.\n\
             \n\
             This derive macro is only needed for instructions that create light accounts \
             (rent-free PDAs, mints, token accounts, or ATAs).\n\
             \n\
             Either:\n\
             1. Add #[light_account(init)] to fields that should be light accounts\n\
             2. Remove #[derive(LightAccounts)] if this instruction doesn't create light accounts",
        ));
    }

    Ok(())
}

/// Validate that required infrastructure fields are present.
fn validate_infra_fields(ctx: &ValidationContext) -> Result<(), syn::Error> {
    // Skip validation if no light_account fields
    if !ctx.has_pdas && !ctx.has_mints && !ctx.has_tokens && !ctx.has_atas {
        return Ok(());
    }

    let mut missing = Vec::new();

    // fee_payer is always required when light_account fields exist
    if !ctx.has_fee_payer {
        missing.push(InfraFieldType::FeePayer);
    }

    // PDAs require compression_config and pda_rent_sponsor
    if ctx.has_pdas {
        if !ctx.has_compression_config {
            missing.push(InfraFieldType::CompressionConfig);
        }
        if !ctx.has_pda_rent_sponsor {
            missing.push(InfraFieldType::PdaRentSponsor);
        }
    }

    // Mints, token accounts (with init), and ATAs (with init) require light_token infrastructure.
    // Mark-only token/ATA fields don't require infrastructure since they don't call CPIs.
    let needs_token_infra = ctx.has_mints || ctx.has_tokens_with_init || ctx.has_atas_with_init;
    if needs_token_infra {
        if !ctx.has_light_token_config {
            missing.push(InfraFieldType::LightTokenConfig);
        }
        if !ctx.has_light_token_rent_sponsor {
            missing.push(InfraFieldType::LightTokenRentSponsor);
        }
        // CPI authority is required for mints only (token init uses program-derived PDA signing)
        if ctx.has_mints && !ctx.has_light_token_cpi_authority {
            missing.push(InfraFieldType::LightTokenCpiAuthority);
        }
    }

    if !missing.is_empty() {
        let mut types = Vec::new();
        if ctx.has_pdas {
            types.push("PDA");
        }
        if ctx.has_mints {
            types.push("mint");
        }
        if ctx.has_tokens {
            types.push("token account");
        }
        if ctx.has_atas {
            types.push("ATA");
        }
        let context = types.join(", ");

        let mut msg = format!(
            "#[derive(LightAccounts)] with {} fields requires the following infrastructure fields:\n",
            context
        );

        for field_type in &missing {
            msg.push_str(&format!(
                "\n  - {} (add one of: {})",
                field_type.description(),
                field_type.accepted_names().join(", ")
            ));
        }

        return Err(syn::Error::new_spanned(ctx.struct_name, msg));
    }

    Ok(())
}

/// Validate that CreateAccountsProof is available when needed.
///
/// CreateAccountsProof is required when there are any init fields (PDAs, mints, tokens, ATAs).
/// It can be provided either:
/// - As a direct argument: `proof: CreateAccountsProof`
/// - As a field on the first instruction arg: `params.create_accounts_proof`
fn validate_proof_availability(ctx: &ValidationContext) -> Result<(), syn::Error> {
    let needs_proof =
        ctx.has_pdas || ctx.has_mints || ctx.has_tokens_with_init || ctx.has_atas_with_init;

    if !needs_proof {
        return Ok(());
    }

    // Check if CreateAccountsProof is available
    if !ctx.has_direct_proof_arg && !ctx.has_instruction_args {
        return Err(syn::Error::new_spanned(
            ctx.struct_name,
            "CreateAccountsProof is required for #[light_account(init)] fields.\n\
             \n\
             Provide it either:\n\
             1. As a direct argument: #[instruction(proof: CreateAccountsProof)]\n\
             2. As a field on params: #[instruction(params: MyParams)] where MyParams has a `create_accounts_proof: CreateAccountsProof` field",
        ));
    }

    Ok(())
}
