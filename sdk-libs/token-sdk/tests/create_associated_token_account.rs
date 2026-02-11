use light_token::instruction::CreateAssociatedTokenAccount;
use solana_pubkey::Pubkey;

const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

#[test]
fn test_discriminator_selection() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ix_regular = CreateAssociatedTokenAccount::new(payer, owner, mint)
        .instruction()
        .unwrap();
    assert_eq!(ix_regular.data[0], CREATE_ATA_DISCRIMINATOR);

    let ix_idempotent = CreateAssociatedTokenAccount::new(payer, owner, mint)
        .idempotent()
        .instruction()
        .unwrap();
    assert_eq!(ix_idempotent.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);
}

#[test]
fn test_instruction_data_consistency() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ix_regular = CreateAssociatedTokenAccount::new(payer, owner, mint)
        .instruction()
        .unwrap();
    let ix_idempotent = CreateAssociatedTokenAccount::new(payer, owner, mint)
        .idempotent()
        .instruction()
        .unwrap();

    assert_eq!(ix_regular.data.len(), ix_idempotent.data.len());
    assert_eq!(ix_regular.data[1..], ix_idempotent.data[1..]);

    assert_eq!(ix_regular.accounts, ix_idempotent.accounts);
    assert_eq!(ix_regular.program_id, ix_idempotent.program_id);
}

#[test]
fn test_account_count() {
    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ix_compressible = CreateAssociatedTokenAccount::new(payer, owner, mint)
        .instruction()
        .unwrap();

    // Account order: owner, mint, payer, ata, system_program, config, rent_sponsor (7 accounts)
    assert_eq!(ix_compressible.accounts.len(), 7);
}
