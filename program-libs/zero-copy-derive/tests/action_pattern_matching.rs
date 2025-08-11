use light_zero_copy::borsh::Deserialize;
use light_zero_copy_derive::ZeroCopy;

// The same Action enum and supporting structs
#[derive(Debug, Clone, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    UpdateMintAuthority(UpdateAuthority),
    UpdateFreezeAuthority(UpdateAuthority),
    CreateSplMint(CreateSplMintAction),
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct Recipient {
    pub recipient: [u8; 32],
    pub amount: u64,
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct MintToAction {
    pub token_account_version: u8,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct UpdateAuthority {
    pub new_authority: Option<[u8; 32]>,
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct CreateSplMintAction {
    pub mint_bump: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching_with_generated_types() {
        // Create test data for MintTo action
        let mut data = vec![0u8]; // discriminant 0 for MintTo
        data.push(1); // token_account_version: 1
        data.push(1); // lamports: Some
        data.extend_from_slice(&5000u64.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 recipient
        let recipient = [42u8; 32];
        data.extend_from_slice(&recipient);
        data.extend_from_slice(&1000u64.to_le_bytes());

        let (action_variant, _remaining) = Action::zero_copy_at(&data).unwrap();

        // The generated enum is ZAction with variants like:
        // - ZAction::MintTo(ZMintToAction)
        // - ZAction::UpdateMintAuthority(ZUpdateAuthority)
        // etc.

        // Here's how you can access the data in practice:
        match action_variant {
            // This is the actual type name that gets generated
            ZAction::MintTo(mint_action) => {
                println!("Processing MintTo action:");

                // Access the token account version (u8 is not wrapped)
                let version = mint_action.token_account_version;
                println!("  Token account version: {}", version);
                assert_eq!(version, 1);

                // Access the lamports (Option<u64>)
                let lamports = mint_action.lamports.as_ref().map(|x| x.get());
                println!("  Lamports: {:?}", lamports);
                assert_eq!(lamports, Some(5000));

                // Access the recipients Vec
                println!("  Recipients count: {}", mint_action.recipients.len());
                assert_eq!(mint_action.recipients.len(), 1);

                // Access individual recipient data
                let first_recipient = &mint_action.recipients[0];
                let recipient_pubkey = first_recipient.recipient.as_ref();
                let amount = first_recipient.amount.get();

                println!(
                    "  First recipient: pubkey={:?}, amount={}",
                    &recipient_pubkey[..4],
                    amount
                ); // Show first 4 bytes
                assert_eq!(recipient_pubkey, &recipient);
                assert_eq!(amount, 1000);
            }
            ZAction::UpdateMintAuthority(update_auth) => {
                println!("Processing UpdateMintAuthority action");
                if let Some(new_auth) = &update_auth.new_authority {
                    println!("  New authority: {:?}", &new_auth.as_ref()[..4]);
                } else {
                    println!("  Revoking authority");
                }
            }
            ZAction::UpdateFreezeAuthority(update_auth) => {
                println!("Processing UpdateFreezeAuthority action");
                if let Some(new_auth) = &update_auth.new_authority {
                    println!("  New freeze authority: {:?}", &new_auth.as_ref()[..4]);
                } else {
                    println!("  Revoking freeze authority");
                }
            }
            ZAction::CreateSplMint(create_spl) => {
                println!("Processing CreateSplMint action");
                println!("  Mint bump: {}", create_spl.mint_bump);
            }
        }
    }

    #[test]
    fn test_update_authority_revoke() {
        // Test UpdateMintAuthority with None (revoke)
        let mut data = vec![1u8]; // discriminant 1
        data.push(0); // None discriminant

        let (action_variant, _) = Action::zero_copy_at(&data).unwrap();

        match action_variant {
            ZAction::UpdateMintAuthority(update_auth) => {
                assert!(update_auth.new_authority.is_none());
                println!("Authority successfully revoked");
            }
            _ => panic!("Expected UpdateMintAuthority variant"),
        }
    }

    #[test]
    fn test_create_spl_mint() {
        // Test CreateSplMint
        let mut data = vec![3u8]; // discriminant 3
        data.push(255); // mint_bump: 255

        let (action_variant, _) = Action::zero_copy_at(&data).unwrap();

        match action_variant {
            ZAction::CreateSplMint(create_spl) => {
                assert_eq!(create_spl.mint_bump, 255);
                println!("CreateSplMint with bump: {}", create_spl.mint_bump);
            }
            _ => panic!("Expected CreateSplMint variant"),
        }
    }

    #[test]
    fn test_complex_mint_to_multiple_recipients() {
        // Test MintTo with multiple recipients and None lamports
        let mut data = vec![0u8]; // discriminant 0
        data.push(2); // token_account_version: 2
        data.push(0); // lamports: None
        data.extend_from_slice(&3u32.to_le_bytes()); // 3 recipients

        // Add 3 recipients with different amounts
        for i in 0..3 {
            let recipient = [i as u8 + 10; 32];
            data.extend_from_slice(&recipient);
            data.extend_from_slice(&((i + 1) * 500_u64).to_le_bytes());
        }

        let (action_variant, _) = Action::zero_copy_at(&data).unwrap();

        match action_variant {
            ZAction::MintTo(mint_action) => {
                assert_eq!(mint_action.token_account_version, 2);
                assert!(mint_action.lamports.is_none());
                assert_eq!(mint_action.recipients.len(), 3);

                println!(
                    "MintTo action with {} recipients:",
                    mint_action.recipients.len()
                );

                for (i, recipient) in mint_action.recipients.iter().enumerate() {
                    let expected_amount = (i + 1) * 500;
                    let actual_amount = recipient.amount.get();

                    println!("  Recipient {}: amount={}", i, actual_amount);
                    assert_eq!(actual_amount, expected_amount as u64);
                    assert_eq!(recipient.recipient.as_ref()[0], i as u8 + 10);
                }
            }
            _ => panic!("Expected MintTo variant"),
        }
    }
}
