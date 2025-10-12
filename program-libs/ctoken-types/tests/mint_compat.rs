// Tests compatibility between Light Protocol BaseCompressedMint and SPL Mint
// Verifies that both implementations correctly serialize/deserialize their data
// and maintain logical equivalence of mint fields.

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_ctoken_types::state::BaseMint;
use rand::{thread_rng, Rng};
use spl_token_2022::{solana_program::program_pack::Pack, state::Mint};

/// Generate random test data for a mint
fn generate_random_mint_data() -> (Option<Pubkey>, Option<Pubkey>, u64, u8, bool) {
    let mut rng = thread_rng();

    // Mint authority - 70% chance of having one
    let mint_authority = if rng.gen_bool(0.7) {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        Some(Pubkey::from(bytes))
    } else {
        None
    };

    // Freeze authority - 50% chance of having one
    let freeze_authority = if rng.gen_bool(0.5) {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        Some(Pubkey::from(bytes))
    } else {
        None
    };

    // Supply - random u64
    let supply = rng.gen::<u64>();

    // Decimals - 0 to 9 (typical range for tokens)
    let decimals = rng.gen_range(0..=9);

    // Is initialized - always true for valid mints
    let is_initialized = true;

    (
        mint_authority,
        freeze_authority,
        supply,
        decimals,
        is_initialized,
    )
}

/// Compare Light and SPL mint structures for logical equivalence
/// Also tests that each format can serialize/deserialize its own data correctly
fn compare_mints(light: &BaseMint, spl: &Mint, iteration: usize) {
    // Compare supply
    assert_eq!(
        light.supply, spl.supply,
        "Supply mismatch at iteration {}",
        iteration
    );

    // Compare decimals
    assert_eq!(
        light.decimals, spl.decimals,
        "Decimals mismatch at iteration {}",
        iteration
    );

    // Compare mint authority
    let light_mint_auth = light.mint_authority.map(|p| p.to_bytes());
    let spl_mint_auth =
        Option::<solana_pubkey::Pubkey>::from(spl.mint_authority).map(|p| p.to_bytes());
    assert_eq!(
        light_mint_auth, spl_mint_auth,
        "Mint authority mismatch at iteration {}",
        iteration
    );

    // Compare freeze authority
    let light_freeze_auth = light.freeze_authority.map(|p| p.to_bytes());
    let spl_freeze_auth =
        Option::<solana_pubkey::Pubkey>::from(spl.freeze_authority).map(|p| p.to_bytes());
    assert_eq!(
        light_freeze_auth, spl_freeze_auth,
        "Freeze authority mismatch at iteration {}",
        iteration
    );

    // Test Light serialization roundtrip
    let light_bytes = light.try_to_vec().unwrap();
    let light_deserialized = BaseMint::try_from_slice(&light_bytes).unwrap();
    assert_eq!(
        light, &light_deserialized,
        "Light mint roundtrip failed at iteration {}",
        iteration
    );

    // Test SPL serialization roundtrip
    let mut spl_bytes = vec![0u8; Mint::LEN];
    Mint::pack(*spl, &mut spl_bytes).unwrap();
    let spl_deserialized = Mint::unpack(&spl_bytes).unwrap();
    assert_eq!(
        spl, &spl_deserialized,
        "SPL mint roundtrip failed at iteration {}",
        iteration
    );

    // Verify serialized sizes are reasonable
    assert!(
        !light_bytes.is_empty() && light_bytes.len() < 200,
        "Light serialized size {} is unreasonable at iteration {}",
        light_bytes.len(),
        iteration
    );
    assert_eq!(
        spl_bytes.len(),
        Mint::LEN,
        "SPL serialized size should be {} at iteration {}",
        Mint::LEN,
        iteration
    );
    assert_eq!(
        light_bytes, spl_bytes,
        "light bytes, spl_bytes {}",
        iteration
    );
    let base_mint_borsh = BaseMint::deserialize(&mut light_bytes.as_slice()).unwrap();
    let mut light_borsh_bytes = Vec::new();
    base_mint_borsh.serialize(&mut light_borsh_bytes).unwrap();
    assert_eq!(
        light_bytes, light_borsh_bytes,
        "light bytes, light_borsh_bytes {}",
        iteration
    );
}

/// Test that borsh serialization of BaseCompressedMint fields matches SPL Mint Pack format
#[test]
fn test_base_mint_borsh_pack_compatibility() {
    for i in 0..1000 {
        // Generate random mint data
        let (mint_authority, freeze_authority, supply, decimals, is_initialized) =
            generate_random_mint_data();

        // Create Light BaseCompressedMint
        // Note: We generate a random mint pubkey for completeness
        let mut spl_mint_bytes = [0u8; 32];
        thread_rng().fill(&mut spl_mint_bytes);

        let light_mint = BaseMint {
            mint_authority,
            supply,
            decimals,
            is_initialized: true,

            freeze_authority,
        };

        // Create SPL Mint
        let mint = Mint {
            mint_authority: mint_authority
                .map(|p| solana_pubkey::Pubkey::from(p.to_bytes()))
                .into(),
            supply,
            decimals,
            is_initialized,
            freeze_authority: freeze_authority
                .map(|p| solana_pubkey::Pubkey::from(p.to_bytes()))
                .into(),
        };

        // Compare the mints
        compare_mints(&light_mint, &mint, i);
    }
}

/// Test edge cases for mint compatibility
#[test]
fn test_mint_edge_cases() {
    // Test 1: No authorities (fixed supply mint)
    let light_no_auth = BaseMint {
        mint_authority: None,
        supply: 1_000_000,
        decimals: 6,
        is_initialized: true,

        freeze_authority: None,
    };

    let spl_no_auth = Mint {
        mint_authority: None.into(),
        supply: 1_000_000,
        decimals: 6,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    compare_mints(&light_no_auth, &spl_no_auth, 0);

    // Test 2: Max values
    let light_max = BaseMint {
        mint_authority: Some(Pubkey::from([255u8; 32])),
        supply: u64::MAX,
        decimals: 9,
        is_initialized: true,

        freeze_authority: Some(Pubkey::from([254u8; 32])),
    };

    let spl_max = Mint {
        mint_authority: Some(solana_pubkey::Pubkey::from([255u8; 32])).into(),
        supply: u64::MAX,
        decimals: 9,
        is_initialized: true,
        freeze_authority: Some(solana_pubkey::Pubkey::from([254u8; 32])).into(),
    };

    compare_mints(&light_max, &spl_max, 1);

    // Test 3: Zero supply mint
    let light_zero = BaseMint {
        mint_authority: Some(Pubkey::from([1u8; 32])),
        supply: 0,
        decimals: 0,
        is_initialized: true,
        freeze_authority: None,
    };

    let spl_zero = Mint {
        mint_authority: Some(solana_pubkey::Pubkey::from([1u8; 32])).into(),
        supply: 0,
        decimals: 0,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    compare_mints(&light_zero, &spl_zero, 2);
}
