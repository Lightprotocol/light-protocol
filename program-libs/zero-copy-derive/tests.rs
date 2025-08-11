use light_zero_copy::borsh::Deserialize;
use light_zero_copy_derive::ZeroCopy;

/// Tests for ZeroCopy derive macro:
/// Functional tests:
/// 1. test_enum_zero_copy_functional - successful enum deserialization with all variants
/// 2. test_struct_zero_copy_functional - successful struct deserialization with all field types
/// Failing tests:
/// 1. test_enum_invalid_discriminant - InvalidConversion error for bad discriminant
/// 2. test_enum_empty_data - ArraySize error for insufficient data
/// 3. test_struct_bounds_violation - proper error handling for invalid field data
/// Randomized tests:
/// 1. test_enum_randomized - 1k iterations with random valid enum data
/// 2. test_struct_randomized - 1k iterations with random struct field combinations

// Test enum definition
#[derive(Debug, Clone, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    UpdateAuthority(UpdateAuthority),
    CreateMint(CreateMintAction),
    Transfer,
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct MintToAction {
    pub amount: u64,
    pub recipient: [u8; 32],
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct UpdateAuthority {
    pub new_authority: Option<[u8; 32]>,
}

#[derive(Debug, Clone, ZeroCopy)]
pub struct CreateMintAction {
    pub decimals: u8,
}

// Test struct definition
#[derive(Debug, Clone, ZeroCopy)]
pub struct TestStruct {
    pub id: u8,
    pub value: u64,
    pub data: [u8; 16],
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;

    #[test]
    fn test_enum_zero_copy_functional() {
        // Test MintTo variant
        let mut data = [0u8; 64];
        let mut offset = 0;
        
        data[offset] = 0; // MintTo discriminant
        offset += 1;
        data[offset..offset + 8].copy_from_slice(&1000u64.to_le_bytes());
        offset += 8;
        data[offset..offset + 32].copy_from_slice(&[42u8; 32]);
        offset += 32;

        let (action, remaining) = Action::zero_copy_at(&data[..offset]).unwrap();
        assert_eq!(remaining.len(), 0);
        match action {
            ZAction::MintTo(mint_action) => {
                assert_eq!(mint_action.amount.get(), 1000u64);
                assert_eq!(mint_action.recipient.as_ref(), &[42u8; 32]);
            }
            _ => panic!("Expected MintTo variant"),
        }

        // Test UpdateAuthority variant with Some
        let mut data2 = [0u8; 64];
        data2[0] = 1; // UpdateAuthority discriminant
        data2[1] = 1; // Some discriminant
        data2[2..34].copy_from_slice(&[99u8; 32]);

        let (action2, remaining2) = Action::zero_copy_at(&data2[..34]).unwrap();
        assert_eq!(remaining2.len(), 0);
        match action2 {
            ZAction::UpdateAuthority(update_auth) => {
                assert!(update_auth.new_authority.is_some());
                assert_eq!(update_auth.new_authority.as_ref().unwrap().as_ref(), &[99u8; 32]);
            }
            _ => panic!("Expected UpdateAuthority variant"),
        }

        // Test CreateMint variant
        let mut data3 = [0u8; 2];
        data3[0] = 2; // CreateMint discriminant
        data3[1] = 9; // decimals

        let (action3, remaining3) = Action::zero_copy_at(&data3).unwrap();
        assert_eq!(remaining3.len(), 0);
        match action3 {
            ZAction::CreateMint(create_mint) => {
                assert_eq!(create_mint.decimals, 9u8);
            }
            _ => panic!("Expected CreateMint variant"),
        }

        // Test Transfer unit variant
        let data4 = [3u8]; // Transfer discriminant
        let (action4, remaining4) = Action::zero_copy_at(&data4).unwrap();
        assert_eq!(remaining4.len(), 0);
        match action4 {
            ZAction::Transfer => {} // Unit variant - just verify it matches
            _ => panic!("Expected Transfer variant"),
        }
    }

    #[test] 
    fn test_struct_zero_copy_functional() {
        let mut data = [0u8; 25];
        data[0] = 5; // id
        data[1..9].copy_from_slice(&12345u64.to_le_bytes()); // value
        data[9..25].copy_from_slice(&[7u8; 16]); // data array

        let (test_struct, remaining) = TestStruct::zero_copy_at(&data).unwrap();
        assert_eq!(remaining.len(), 0);
        
        // Verify complete struct data
        assert_eq!(test_struct.id, 5u8);
        assert_eq!(test_struct.value.get(), 12345u64);
        assert_eq!(test_struct.data.as_ref(), &[7u8; 16]);
    }

    #[test]
    fn test_enum_invalid_discriminant() {
        let data = [99u8]; // Invalid discriminant
        let result = Action::zero_copy_at(&data);
        assert_eq!(result.unwrap_err(), light_zero_copy::errors::ZeroCopyError::InvalidConversion);
    }

    #[test]
    fn test_enum_empty_data() {
        let data = [];
        let result = Action::zero_copy_at(&data);
        match result.unwrap_err() {
            light_zero_copy::errors::ZeroCopyError::ArraySize(expected, actual) => {
                assert_eq!(expected, 1);
                assert_eq!(actual, 0);
            }
            other => panic!("Expected ArraySize error, got: {:?}", other),
        }
    }

    #[test]
    fn test_struct_bounds_violation() {
        // Test insufficient data for struct
        let data = [1u8, 2u8]; // Only 2 bytes, need 25
        let result = TestStruct::zero_copy_at(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_randomized() {
        let mut rng = StdRng::seed_from_u64(0);

        for _ in 0..1000 {
            let variant = rng.gen_range(0..4u8);
            let mut data = [0u8; 64];
            data[0] = variant;

            let size = match variant {
                0 => { // MintTo
                    let amount: u64 = rng.gen();
                    let recipient: [u8; 32] = rng.gen();
                    data[1..9].copy_from_slice(&amount.to_le_bytes());
                    data[9..41].copy_from_slice(&recipient);
                    41
                }
                1 => { // UpdateAuthority 
                    if rng.gen_bool(0.5) {
                        data[1] = 1; // Some
                        let authority: [u8; 32] = rng.gen();
                        data[2..34].copy_from_slice(&authority);
                        34
                    } else {
                        data[1] = 0; // None
                        2
                    }
                }
                2 => { // CreateMint
                    data[1] = rng.gen::<u8>(); // decimals
                    2
                }
                3 => { // Transfer (unit)
                    1
                }
                _ => unreachable!(),
            };

            let (action, remaining) = Action::zero_copy_at(&data[..size]).unwrap();
            assert_eq!(remaining.len(), 0);
            
            // Verify data matches what we put in
            match (variant, action) {
                (0, ZAction::MintTo(mint_action)) => {
                    let expected_amount = u64::from_le_bytes([data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]]);
                    let expected_recipient = &data[9..41];
                    assert_eq!(mint_action.amount.get(), expected_amount);
                    assert_eq!(mint_action.recipient.as_ref(), expected_recipient);
                }
                (1, ZAction::UpdateAuthority(update_auth)) => {
                    if data[1] == 1 {
                        assert!(update_auth.new_authority.is_some());
                        let expected_auth = &data[2..34];
                        assert_eq!(update_auth.new_authority.as_ref().unwrap().as_ref(), expected_auth);
                    } else {
                        assert!(update_auth.new_authority.is_none());
                    }
                }
                (2, ZAction::CreateMint(create_mint)) => {
                    assert_eq!(create_mint.decimals, data[1]);
                }
                (3, ZAction::Transfer) => {
                    // Unit variant - nothing to verify
                }
                _ => panic!("Variant mismatch"),
            }
        }
    }

    #[test]
    fn test_struct_randomized() {
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..1000 {
            let id: u8 = rng.gen();
            let value: u64 = rng.gen();
            let data_array: [u8; 16] = rng.gen();

            let mut data = [0u8; 25];
            data[0] = id;
            data[1..9].copy_from_slice(&value.to_le_bytes());
            data[9..25].copy_from_slice(&data_array);

            let (test_struct, remaining) = TestStruct::zero_copy_at(&data).unwrap();
            assert_eq!(remaining.len(), 0);
            
            // Verify complete struct data matches input
            assert_eq!(test_struct.id, id);
            assert_eq!(test_struct.value.get(), value);
            assert_eq!(test_struct.data.as_ref(), &data_array);
        }
    }
}