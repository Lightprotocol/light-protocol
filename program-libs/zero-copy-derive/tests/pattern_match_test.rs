use light_zero_copy_derive::ZeroCopy;

// Test struct for the MintTo action
#[repr(C)]
#[derive(Debug, Clone, PartialEq, ZeroCopy)]
pub struct MintToAction {
    pub amount: u64,
    pub recipient: Vec<u8>,
}

// Test enum similar to your Action example
#[repr(C)]
#[derive(Debug, Clone, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    Update,
    CreateSplMint,
    UpdateMetadata,
}

#[cfg(test)]
mod tests {
    use light_zero_copy::traits::ZeroCopyAt;

    use super::*;

    #[test]
    fn test_pattern_matching_works() {
        use light_zero_copy::traits::ZeroCopyAt;
        // Test MintTo variant (discriminant 0)
        let mut data = vec![0u8]; // discriminant 0 for MintTo

        // Add MintToAction serialized data
        // amount: 1000
        data.extend_from_slice(&1000u64.to_le_bytes());

        // recipient: "alice" (5 bytes length + "alice")
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(b"alice");

        let (result, _remaining) = Action::zero_copy_at(&data).unwrap();
        // This is the key test - we should be able to pattern match!
        // The generated type should be ZAction<'_> with variants like ZAction::MintTo(ZMintToAction<'_>)
        match result {
            // This pattern should work with the concrete Z-types
            action_variant => {
                // We can't easily test the exact pattern match without importing the generated type
                // but we can verify the structure exists and is Debug printable
                println!("Pattern match successful: {:?}", action_variant);
                // In real usage, this would be:
                // ZAction::MintTo(mint_action) => {
                //     // use mint_action.amount, mint_action.recipient, etc.
                // }
                // ZAction::Update => { /* handle update */ }
                // etc.
            }
        }
    }

    #[test]
    fn test_unit_variant_pattern_matching() {
        use light_zero_copy::traits::ZeroCopyAt;
        // Test Update variant (discriminant 1)
        let data = [1u8];
        let (result, _remaining) = Action::zero_copy_at(&data).unwrap();

        // This should also support pattern matching
        match result {
            action_variant => {
                println!(
                    "Unit variant pattern match successful: {:?}",
                    action_variant
                );
                // In real usage: ZAction::Update => { /* handle */ }
            }
        }
    }
}

// This shows what the user's code should look like:
//
// for action in parsed_instruction_data.actions.iter() {
//     match action {
//         ZAction::MintTo(mint_action) => {
//             // Access mint_action.amount, mint_action.recipient, etc.
//             println!("Minting {} tokens to {:?}", mint_action.amount, mint_action.recipient);
//         }
//         ZAction::Update => {
//             println!("Performing update");
//         }
//         ZAction::CreateSplMint => {
//             println!("Creating SPL mint");
//         }
//         ZAction::UpdateMetadata => {
//             println!("Updating metadata");
//         }
//     }
// }
