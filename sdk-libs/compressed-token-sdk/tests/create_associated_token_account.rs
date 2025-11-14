use light_compressed_token_sdk::instructions::create_associated_token_account::*;
use solana_pubkey::Pubkey;

/// Discriminators for create ATA instructions
const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

#[test]
fn test_discriminator_selection() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Test non-idempotent variant
    let ix_regular = create_associated_token_account(payer, owner, mint).unwrap();
    assert_eq!(ix_regular.data[0], CREATE_ATA_DISCRIMINATOR);

    // Test idempotent variant
    let ix_idempotent = create_associated_token_account_idempotent(payer, owner, mint).unwrap();
    assert_eq!(ix_idempotent.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);

    // Test generic with false
    let ix_generic_false =
        create_associated_token_account_with_mode::<false>(payer, owner, mint).unwrap();
    assert_eq!(ix_generic_false.data[0], CREATE_ATA_DISCRIMINATOR);

    // Test generic with true
    let ix_generic_true =
        create_associated_token_account_with_mode::<true>(payer, owner, mint).unwrap();
    assert_eq!(ix_generic_true.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);
}

#[test]
fn test_compressible_discriminator_selection() {
    let inputs = CreateCompressibleAssociatedTokenAccountInputs {
        payer: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        mint: Pubkey::new_unique(),
        rent_sponsor: Pubkey::new_unique(),
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compressible_config: Pubkey::new_unique(),
        token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
    };

    // Test non-idempotent variant
    let ix_regular = create_compressible_associated_token_account(inputs.clone()).unwrap();
    assert_eq!(ix_regular.data[0], CREATE_ATA_DISCRIMINATOR);

    // Test idempotent variant
    let ix_idempotent =
        create_compressible_associated_token_account_idempotent(inputs.clone()).unwrap();
    assert_eq!(ix_idempotent.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);

    // Test generic with false
    let ix_generic_false =
        create_compressible_associated_token_account_with_mode::<false>(inputs.clone()).unwrap();
    assert_eq!(ix_generic_false.data[0], CREATE_ATA_DISCRIMINATOR);

    // Test generic with true
    let ix_generic_true =
        create_compressible_associated_token_account_with_mode::<true>(inputs).unwrap();
    assert_eq!(ix_generic_true.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);
}

#[test]
fn test_instruction_data_consistency() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Create both variants
    let ix_regular = create_associated_token_account(payer, owner, mint).unwrap();
    let ix_idempotent = create_associated_token_account_idempotent(payer, owner, mint).unwrap();

    // Both should have same data except for discriminator
    assert_eq!(ix_regular.data.len(), ix_idempotent.data.len());
    assert_eq!(ix_regular.data[1..], ix_idempotent.data[1..]);

    // Accounts should be identical
    assert_eq!(ix_regular.accounts, ix_idempotent.accounts);
    assert_eq!(ix_regular.program_id, ix_idempotent.program_id);
}

#[test]
fn test_ata_address_derivation() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let (ata_pubkey, _bump) = derive_ctoken_ata(&owner, &mint);

    // Verify that the derived ATA address is consistent
    let (ata_pubkey2, _bump2) = derive_ctoken_ata(&owner, &mint);
    assert_eq!(
        ata_pubkey, ata_pubkey2,
        "ATA derivation should be deterministic"
    );

    // Verify that different owners produce different ATAs
    let owner2 = Pubkey::new_unique();
    let (ata_pubkey3, _bump3) = derive_ctoken_ata(&owner2, &mint);
    assert_ne!(
        ata_pubkey, ata_pubkey3,
        "Different owners should have different ATAs"
    );

    // Verify that different mints produce different ATAs
    let mint2 = Pubkey::new_unique();
    let (ata_pubkey4, _bump4) = derive_ctoken_ata(&owner, &mint2);
    assert_ne!(
        ata_pubkey, ata_pubkey4,
        "Different mints should have different ATAs"
    );
}
