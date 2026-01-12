#![cfg(feature = "compressible")]

use light_sdk::instruction::PackedAccounts;
use light_token_sdk::{
    compat::{PackedCTokenDataWithVariant, TokenData, TokenDataWithVariant},
    pack::Pack,
};
use solana_pubkey::Pubkey;

#[test]
fn test_token_data_packing() {
    let mut remaining_accounts = PackedAccounts::default();

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();

    let token_data = TokenData {
        owner,
        mint,
        amount: 1000,
        delegate: Some(delegate),
        state: Default::default(),
        tlv: None,
    };

    // Pack the token data
    let packed = token_data.pack(&mut remaining_accounts);

    // Verify the packed data
    assert_eq!(packed.owner, 0); // First pubkey gets index 0
    assert_eq!(packed.mint, 1); // Second pubkey gets index 1
    assert_eq!(packed.delegate, 2); // Third pubkey gets index 2
    assert_eq!(packed.amount, 1000);
    assert!(packed.has_delegate);
    assert_eq!(packed.version, 3); // TokenDataVersion::ShaFlat

    // Verify remaining_accounts contains the pubkeys
    let pubkeys = remaining_accounts.packed_pubkeys();
    assert_eq!(pubkeys[0], owner);
    assert_eq!(pubkeys[1], mint);
    assert_eq!(pubkeys[2], delegate);
}

#[test]
fn test_token_data_with_variant_packing() {
    use anchor_lang::{AnchorDeserialize, AnchorSerialize};

    #[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize)]
    enum MyVariant {
        TypeA = 0,
        TypeB = 1,
    }

    let mut remaining_accounts = PackedAccounts::default();

    let token_with_variant = TokenDataWithVariant {
        variant: MyVariant::TypeA,
        token_data: TokenData {
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            amount: 500,
            delegate: None,
            state: Default::default(),
            tlv: None,
        },
    };

    // Pack the wrapper
    let packed: PackedCTokenDataWithVariant<MyVariant> =
        token_with_variant.pack(&mut remaining_accounts);

    // Verify variant is unchanged
    assert!(matches!(packed.variant, MyVariant::TypeA));

    // Verify token data is packed
    assert_eq!(packed.token_data.owner, 0);
    assert_eq!(packed.token_data.mint, 1);
    assert_eq!(packed.token_data.amount, 500);
    assert!(!packed.token_data.has_delegate);
}

#[test]
fn test_deduplication_in_packing() {
    let mut remaining_accounts = PackedAccounts::default();

    let shared_owner = Pubkey::new_unique();
    let shared_mint = Pubkey::new_unique();

    let token1 = TokenData {
        owner: shared_owner,
        mint: shared_mint,
        amount: 100,
        delegate: None,
        state: Default::default(),
        tlv: None,
    };

    let token2 = TokenData {
        owner: shared_owner, // Same owner
        mint: shared_mint,   // Same mint
        amount: 200,
        delegate: None,
        state: Default::default(),
        tlv: None,
    };

    // Pack both tokens
    let packed1 = token1.pack(&mut remaining_accounts);
    let packed2 = token2.pack(&mut remaining_accounts);

    // Both should reference the same indices
    assert_eq!(packed1.owner, packed2.owner);
    assert_eq!(packed1.mint, packed2.mint);

    // Only 2 unique pubkeys should be stored
    let pubkeys = remaining_accounts.packed_pubkeys();
    assert_eq!(pubkeys.len(), 2);
}
