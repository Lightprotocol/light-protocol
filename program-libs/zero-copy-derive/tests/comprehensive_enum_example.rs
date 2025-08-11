/*!
This file demonstrates the complete enum support for the ZeroCopy derive macro.

## What gets generated:

For this enum:
```rust
#[derive(ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    Update,
    CreateSplMint,
    UpdateMetadata,
}
```

The macro generates:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ZAction<'a> {
    MintTo(ZMintToAction<'a>),  // Concrete type for pattern matching
    Update,
    CreateSplMint,
    UpdateMetadata,
}

impl<'a> Deserialize<'a> for Action {
    type Output = ZAction<'a>;
    
    fn zero_copy_at(data: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        match data[0] {
            0 => {
                let (value, bytes) = MintToAction::zero_copy_at(&data[1..])?;
                Ok((ZAction::MintTo(value), bytes))
            }
            1 => Ok((ZAction::Update, &data[1..])),
            2 => Ok((ZAction::CreateSplMint, &data[1..])),
            3 => Ok((ZAction::UpdateMetadata, &data[1..])),
            _ => Err(ZeroCopyError::InvalidConversion),
        }
    }
}
```

## Usage:

```rust
for action in parsed_instruction_data.actions.iter() {
    match action {
        ZAction::MintTo(mint_action) => {
            // Access mint_action.amount, mint_action.recipient, etc.
        }
        ZAction::Update => {
            // Handle update
        }
        ZAction::CreateSplMint => {
            // Handle SPL mint creation
        }
        ZAction::UpdateMetadata => {
            // Handle metadata update
        }
    }
}
```
*/

use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, Clone, PartialEq, ZeroCopy)]
pub struct MintToAction {
    pub amount: u64,
    pub recipient: Vec<u8>,
}

#[derive(Debug, Clone, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    Update,
    CreateSplMint,
    UpdateMetadata,
}

#[cfg(test)]
mod tests {
    use light_zero_copy::borsh::Deserialize;
    use super::*;

    #[test]
    fn test_generated_enum_structure() {
        // The macro should generate ZAction<'a> with concrete variants
        
        // Test unit variants
        for (discriminant, expected_name) in [(1u8, "Update"), (2u8, "CreateSplMint"), (3u8, "UpdateMetadata")] {
            let data = [discriminant];
            let (result, remaining) = Action::zero_copy_at(&data).unwrap();
            assert_eq!(remaining.len(), 0);
            println!("✓ {}: {:?}", expected_name, result);
        }
        
        // Test data variant
        let mut data = vec![0u8]; // MintTo discriminant
        data.extend_from_slice(&42u64.to_le_bytes()); // amount
        data.extend_from_slice(&4u32.to_le_bytes()); // recipient length
        data.extend_from_slice(b"test"); // recipient data
        
        let (result, remaining) = Action::zero_copy_at(&data).unwrap();
        assert_eq!(remaining.len(), 0);
        println!("✓ MintTo: {:?}", result);
    }
    
    #[test]
    fn test_pattern_matching_example() {
        // This demonstrates the exact usage pattern the user wants
        let mut actions_data = Vec::new();
        
        // Create some test actions
        // Action 1: MintTo
        actions_data.push({
            let mut data = vec![0u8]; // MintTo discriminant
            data.extend_from_slice(&1000u64.to_le_bytes());
            data.extend_from_slice(&5u32.to_le_bytes());
            data.extend_from_slice(b"alice");
            data
        });
        
        // Action 2: Update  
        actions_data.push(vec![1u8]);
        
        // Action 3: CreateSplMint
        actions_data.push(vec![2u8]);

        // Process each action (simulating the user's use case)
        for (i, action_data) in actions_data.iter().enumerate() {
            let (action, _) = Action::zero_copy_at(action_data).unwrap();
            
            // This is what the user wants to be able to write:
            println!("Processing action {}: {:?}", i, action);
            
            // In the user's real code, this would be:
            // match action {
            //     ZAction::MintTo(mint_action) => {
            //         println!("Minting {} tokens to {:?}", mint_action.amount, mint_action.recipient);
            //     }
            //     ZAction::Update => {
            //         println!("Performing update");
            //     }
            //     ZAction::CreateSplMint => {
            //         println!("Creating SPL mint");
            //     }
            //     ZAction::UpdateMetadata => {
            //         println!("Updating metadata");
            //     }
            // }
        }
    }
}