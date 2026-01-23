//! Unit tests for #[light_account(...)] attribute parsing.
//!
//! Extracted from `light_pdas/accounts/light_account.rs`.

use syn::parse_quote;

use crate::light_pdas::accounts::light_account::{parse_light_account_attr, LightAccountField};

#[test]
fn test_parse_light_account_pda_bare() {
    let field: syn::Field = parse_quote! {
        #[light_account(init)]
        pub record: Account<'info, MyRecord>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
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
fn test_parse_pda_tree_keywords_rejected() {
    // Tree keywords are no longer allowed - they're auto-fetched from CreateAccountsProof
    let field: syn::Field = parse_quote! {
        #[light_account(init, pda::address_tree_info = custom_tree)]
        pub record: Account<'info, MyRecord>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
}

#[test]
fn test_parse_light_account_mint() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"test"]
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
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
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"test"],
            mint::name = params.name.clone(),
            mint::symbol = params.symbol.clone(),
            mint::uri = params.uri.clone()
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
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
        #[light_account(mint, mint::decimals = 9)]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
}

#[test]
fn test_parse_light_account_mint_missing_required() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint, mint::decimals = 9)]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
}

#[test]
fn test_parse_light_account_partial_metadata_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"test"],
            mint::name = params.name.clone()
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
}

#[test]
fn test_no_light_account_attr_returns_none() {
    let field: syn::Field = parse_quote! {
        pub record: Account<'info, MyRecord>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ========================================================================
// Token Account Tests
// ========================================================================

#[test]
fn test_parse_token_mark_only_returns_none() {
    // Mark-only mode (no init) should return None for LightAccounts derive
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"authority"])]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_token_init_creates_field() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, token, token::authority = [b"authority"], token::mint = token_mint, token::owner = vault_authority)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::TokenAccount(token) => {
            assert_eq!(token.field_ident.to_string(), "vault");
            assert!(token.has_init);
            assert!(!token.authority_seeds.is_empty());
            assert!(token.mint.is_some());
            assert!(token.owner.is_some());
        }
        _ => panic!("Expected TokenAccount field"),
    }
}

#[test]
fn test_parse_token_init_missing_authority_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, token)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("authority"));
}

#[test]
fn test_parse_token_mark_only_missing_authority_fails() {
    // Mark-only token now requires authority
    let field: syn::Field = parse_quote! {
        #[light_account(token)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("authority"),
        "Expected error about missing authority, got: {}",
        err
    );
}

#[test]
fn test_parse_token_mark_only_rejects_mint() {
    // Mark-only token should not allow mint parameter
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"auth"], token::mint = token_mint)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("mint") && err.contains("only allowed with `init`"),
        "Expected error about mint only for init, got: {}",
        err
    );
}

#[test]
fn test_parse_token_mark_only_rejects_owner() {
    // Mark-only token should not allow owner parameter
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"auth"], token::owner = vault_authority)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("owner") && err.contains("only allowed with `init`"),
        "Expected error about owner only for init, got: {}",
        err
    );
}

#[test]
fn test_parse_token_init_missing_mint_fails() {
    // Token init requires mint parameter
    let field: syn::Field = parse_quote! {
        #[light_account(init, token, token::authority = [b"authority"], token::owner = vault_authority)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("mint"),
        "Expected error about missing mint, got: {}",
        err
    );
}

#[test]
fn test_parse_token_init_missing_owner_fails() {
    // Token init requires owner parameter
    let field: syn::Field = parse_quote! {
        #[light_account(init, token, token::authority = [b"authority"], token::mint = token_mint)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("owner"),
        "Expected error about missing owner, got: {}",
        err
    );
}

// ========================================================================
// Associated Token Tests
// ========================================================================

#[test]
fn test_parse_associated_token_mark_only_returns_none() {
    // Mark-only mode (no init) should return None for LightAccounts derive
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::authority = owner, associated_token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_associated_token_init_creates_field() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, associated_token, associated_token::authority = owner, associated_token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::AssociatedToken(ata) => {
            assert_eq!(ata.field_ident.to_string(), "user_ata");
            assert!(ata.has_init);
        }
        _ => panic!("Expected AssociatedToken field"),
    }
}

#[test]
fn test_parse_associated_token_init_missing_authority_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, associated_token, associated_token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("authority"));
}

#[test]
fn test_parse_associated_token_init_missing_mint_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(init, associated_token, associated_token::authority = owner)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("mint"));
}

#[test]
fn test_parse_token_unknown_argument_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"auth"], token::unknown = foo)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("unknown"));
}

#[test]
fn test_parse_associated_token_unknown_argument_fails() {
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::authority = owner, associated_token::mint = mint, associated_token::unknown = foo)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("unknown"));
}

#[test]
fn test_parse_associated_token_shorthand_syntax() {
    // Test shorthand syntax: mint, authority, bump without = value
    let field: syn::Field = parse_quote! {
        #[light_account(init, associated_token, associated_token::authority, associated_token::mint, associated_token::bump)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::AssociatedToken(ata) => {
            assert_eq!(ata.field_ident.to_string(), "user_ata");
            assert!(ata.has_init);
            assert!(ata.bump.is_some());
        }
        _ => panic!("Expected AssociatedToken field"),
    }
}

#[test]
fn test_parse_token_duplicate_key_fails() {
    // Duplicate keys should be rejected
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"auth1"], token::authority = [b"auth2"])]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Duplicate key"),
        "Expected error about duplicate key, got: {}",
        err
    );
}

#[test]
fn test_parse_associated_token_duplicate_key_fails() {
    // Duplicate keys in associated_token should also be rejected
    let field: syn::Field = parse_quote! {
        #[light_account(init, associated_token, associated_token::authority = foo, associated_token::authority = bar, associated_token::mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Duplicate key"),
        "Expected error about duplicate key, got: {}",
        err
    );
}

#[test]
fn test_parse_token_init_empty_authority_fails() {
    // Empty authority seeds with init should be rejected
    let field: syn::Field = parse_quote! {
        #[light_account(init, token, token::authority = [], token::mint = token_mint, token::owner = vault_authority)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Empty authority seeds"),
        "Expected error about empty authority seeds, got: {}",
        err
    );
}

#[test]
fn test_parse_token_non_init_empty_authority_allowed() {
    // Empty authority seeds without init should be allowed (mark-only mode)
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [])]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    // Mark-only mode returns Ok(None)
    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_pda_with_direct_proof_arg_uses_proof_ident_for_defaults() {
    use syn::Ident;
    // When CreateAccountsProof is passed as a direct instruction arg (not nested in params),
    // the default address_tree_info and output_tree should reference the proof arg directly.
    let field: syn::Field = parse_quote! {
        #[light_account(init)]
        pub record: Account<'info, MyRecord>
    };
    let field_ident = field.ident.clone().unwrap();

    // Simulate passing CreateAccountsProof as direct arg named "proof"
    let proof_ident: Ident = parse_quote!(proof);
    let direct_proof_arg = Some(proof_ident.clone());

    let result = parse_light_account_attr(&field, &field_ident, &direct_proof_arg);
    assert!(
        result.is_ok(),
        "Should parse successfully with direct proof arg"
    );
    let result = result.unwrap();
    assert!(result.is_some(), "Should return Some for init PDA");

    match result.unwrap() {
        LightAccountField::Pda(pda) => {
            assert_eq!(pda.ident.to_string(), "record");

            // Verify defaults use the direct proof identifier
            // address_tree_info should be: proof.address_tree_info
            let addr_tree_info = &pda.address_tree_info;
            let addr_tree_str = quote::quote!(#addr_tree_info).to_string();
            assert!(
                addr_tree_str.contains("proof"),
                "address_tree_info should reference 'proof', got: {}",
                addr_tree_str
            );
            assert!(
                addr_tree_str.contains("address_tree_info"),
                "address_tree_info should access .address_tree_info field, got: {}",
                addr_tree_str
            );

            // output_tree should be: proof.output_state_tree_index
            let output_tree = &pda.output_tree;
            let output_tree_str = quote::quote!(#output_tree).to_string();
            assert!(
                output_tree_str.contains("proof"),
                "output_tree should reference 'proof', got: {}",
                output_tree_str
            );
            assert!(
                output_tree_str.contains("output_state_tree_index"),
                "output_tree should access .output_state_tree_index field, got: {}",
                output_tree_str
            );
        }
        _ => panic!("Expected PDA field"),
    }
}

#[test]
fn test_parse_mint_with_direct_proof_arg_uses_proof_ident_for_defaults() {
    use syn::Ident;
    // When CreateAccountsProof is passed as a direct instruction arg,
    // the default address_tree_info should reference the proof arg directly.
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"test"]
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let field_ident = field.ident.clone().unwrap();

    // Simulate passing CreateAccountsProof as direct arg named "create_proof"
    let proof_ident: Ident = parse_quote!(create_proof);
    let direct_proof_arg = Some(proof_ident.clone());

    let result = parse_light_account_attr(&field, &field_ident, &direct_proof_arg);
    assert!(
        result.is_ok(),
        "Should parse successfully with direct proof arg"
    );
    let result = result.unwrap();
    assert!(result.is_some(), "Should return Some for init mint");

    match result.unwrap() {
        LightAccountField::Mint(mint) => {
            assert_eq!(mint.field_ident.to_string(), "cmint");

            // Verify default address_tree_info uses the direct proof identifier
            // Should be: create_proof.address_tree_info
            let addr_tree_info = &mint.address_tree_info;
            let addr_tree_str = quote::quote!(#addr_tree_info).to_string();
            assert!(
                addr_tree_str.contains("create_proof"),
                "address_tree_info should reference 'create_proof', got: {}",
                addr_tree_str
            );
            assert!(
                addr_tree_str.contains("address_tree_info"),
                "address_tree_info should access .address_tree_info field, got: {}",
                addr_tree_str
            );

            // Verify default output_tree uses the direct proof identifier
            // Should be: create_proof.output_state_tree_index
            let output_tree = &mint.output_tree;
            let output_tree_str = quote::quote!(#output_tree).to_string();
            assert!(
                output_tree_str.contains("create_proof"),
                "output_tree should reference 'create_proof', got: {}",
                output_tree_str
            );
            assert!(
                output_tree_str.contains("output_state_tree_index"),
                "output_tree should access .output_state_tree_index field, got: {}",
                output_tree_str
            );
        }
        _ => panic!("Expected Mint field"),
    }
}

// ========================================================================
// Bump Parameter Tests
// ========================================================================

#[test]
fn test_parse_token_with_bump_parameter() {
    // Test token with explicit bump parameter
    let field: syn::Field = parse_quote! {
        #[light_account(init, token,
            token::authority = [b"vault", self.offer.key()],
            token::mint = token_mint,
            token::owner = vault_authority,
            token::bump = params.vault_bump
        )]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully with bump parameter"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::TokenAccount(token) => {
            assert_eq!(token.field_ident.to_string(), "vault");
            assert!(token.has_init);
            assert!(!token.authority_seeds.is_empty());
            assert!(token.bump.is_some(), "bump should be Some when provided");
        }
        _ => panic!("Expected TokenAccount field"),
    }
}

#[test]
fn test_parse_token_without_bump_backwards_compatible() {
    // Test token without bump (backwards compatible - bump will be auto-derived)
    let field: syn::Field = parse_quote! {
        #[light_account(init, token,
            token::authority = [b"vault", self.offer.key()],
            token::mint = token_mint,
            token::owner = vault_authority
        )]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully without bump parameter"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::TokenAccount(token) => {
            assert_eq!(token.field_ident.to_string(), "vault");
            assert!(token.has_init);
            assert!(!token.authority_seeds.is_empty());
            assert!(
                token.bump.is_none(),
                "bump should be None when not provided"
            );
        }
        _ => panic!("Expected TokenAccount field"),
    }
}

#[test]
fn test_parse_mint_with_mint_bump() {
    // Test mint with explicit mint::bump parameter
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"mint"],
            mint::bump = params.mint_bump
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully with mint::bump parameter"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::Mint(mint) => {
            assert_eq!(mint.field_ident.to_string(), "cmint");
            assert!(
                mint.mint_bump.is_some(),
                "mint_bump should be Some when provided"
            );
        }
        _ => panic!("Expected Mint field"),
    }
}

#[test]
fn test_parse_mint_with_authority_bump() {
    // Test mint with authority_seeds and authority_bump
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"mint"],
            mint::authority_seeds = &[b"auth"],
            mint::authority_bump = params.auth_bump
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully with authority_bump parameter"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::Mint(mint) => {
            assert_eq!(mint.field_ident.to_string(), "cmint");
            assert!(
                mint.authority_seeds.is_some(),
                "authority_seeds should be Some"
            );
            assert!(
                mint.authority_bump.is_some(),
                "authority_bump should be Some when provided"
            );
        }
        _ => panic!("Expected Mint field"),
    }
}

#[test]
fn test_parse_mint_without_bumps_backwards_compatible() {
    // Test mint without bump parameters (backwards compatible - bumps will be auto-derived)
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint,
            mint::signer = mint_signer,
            mint::authority = authority,
            mint::decimals = 9,
            mint::seeds = &[b"mint"],
            mint::authority_seeds = &[b"auth"]
        )]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully without bump parameters"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::Mint(mint) => {
            assert_eq!(mint.field_ident.to_string(), "cmint");
            assert!(
                mint.mint_bump.is_none(),
                "mint_bump should be None when not provided"
            );
            assert!(
                mint.authority_seeds.is_some(),
                "authority_seeds should be Some"
            );
            assert!(
                mint.authority_bump.is_none(),
                "authority_bump should be None when not provided"
            );
        }
        _ => panic!("Expected Mint field"),
    }
}

#[test]
fn test_parse_token_bump_shorthand_syntax() {
    // Test token with bump shorthand syntax (token::bump = bump)
    let field: syn::Field = parse_quote! {
        #[light_account(init, token,
            token::authority = [b"vault"],
            token::mint = token_mint,
            token::owner = vault_authority,
            token::bump
        )]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(
        result.is_ok(),
        "Should parse successfully with bump shorthand"
    );
    let result = result.unwrap();
    assert!(result.is_some());

    match result.unwrap() {
        LightAccountField::TokenAccount(token) => {
            assert!(
                token.bump.is_some(),
                "bump should be Some with shorthand syntax"
            );
        }
        _ => panic!("Expected TokenAccount field"),
    }
}

// ========================================================================
// Namespace Validation Tests
// ========================================================================

#[test]
fn test_parse_wrong_namespace_fails() {
    // Using mint:: namespace with token account type should fail
    let field: syn::Field = parse_quote! {
        #[light_account(token, mint::authority = [b"auth"])]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("doesn't match account type"),
        "Expected namespace mismatch error, got: {}",
        err
    );
}

#[test]
fn test_old_syntax_gives_helpful_error() {
    // Old syntax without namespace should give helpful migration error
    let field: syn::Field = parse_quote! {
        #[light_account(init, mint, authority = some_authority)]
        pub cmint: UncheckedAccount<'info>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Missing namespace prefix") || err.contains("mint::authority"),
        "Expected helpful migration error, got: {}",
        err
    );
}

// ========================================================================
// Mark-Only Associated Token Validation Tests
// ========================================================================

#[test]
fn test_parse_associated_token_mark_only_missing_authority_fails() {
    // Mark-only associated_token requires authority
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("authority"),
        "Expected error about missing authority, got: {}",
        err
    );
}

#[test]
fn test_parse_associated_token_mark_only_missing_mint_fails() {
    // Mark-only associated_token requires mint
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::authority = owner)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("mint"),
        "Expected error about missing mint, got: {}",
        err
    );
}

#[test]
fn test_parse_associated_token_mark_only_with_both_params_succeeds() {
    // Mark-only associated_token with both authority and mint should succeed (returns None)
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::authority = owner, associated_token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Mark-only returns None
}

// ========================================================================
// Mixed Namespace Prefix Tests
// ========================================================================

#[test]
fn test_parse_mixed_token_and_associated_token_prefix_fails() {
    // Mixing token:: with associated_token type should fail
    let field: syn::Field = parse_quote! {
        #[light_account(associated_token, associated_token::authority = owner, token::mint = mint)]
        pub user_ata: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("doesn't match account type"),
        "Expected namespace mismatch error, got: {}",
        err
    );
}

#[test]
fn test_parse_mixed_associated_token_and_token_prefix_fails() {
    // Mixing associated_token:: with token type should fail
    let field: syn::Field = parse_quote! {
        #[light_account(token, token::authority = [b"auth"], associated_token::mint = mint)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("doesn't match account type"),
        "Expected namespace mismatch error, got: {}",
        err
    );
}

#[test]
fn test_parse_init_mixed_token_and_mint_prefix_fails() {
    // Mixing token:: with mint:: in init mode should fail
    let field: syn::Field = parse_quote! {
        #[light_account(init, token, token::authority = [b"auth"], mint::decimals = 9)]
        pub vault: Account<'info, CToken>
    };
    let ident = field.ident.clone().unwrap();

    let result = parse_light_account_attr(&field, &ident, &None);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("doesn't match account type"),
        "Expected namespace mismatch error, got: {}",
        err
    );
}
