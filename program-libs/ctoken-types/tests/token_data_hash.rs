use light_compressed_account::Pubkey;
use light_ctoken_types::state::{CompressedTokenAccountState, TokenData};

pub struct TestCase {
    pub name: String,
    pub token_data: TokenData,
    pub hash_v1: [u8; 32],
    pub hash_v2: [u8; 32],
    pub hash_v3: [u8; 32],
}

#[test]
fn token_data_constant_reference_hashes() {
    let mint_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);
    let owner_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);
    let delegate_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);

    let test_cases = [
        TestCase {
            name: "01_max_amount_initialized_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: u64::MAX,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                2, 254, 95, 117, 84, 161, 103, 204, 178, 14, 147, 115, 110, 124, 228, 160, 9, 1,
                130, 199, 252, 198, 46, 123, 230, 129, 186, 16, 161, 138, 134, 213,
            ],
            hash_v1: [
                2, 254, 95, 117, 84, 161, 103, 204, 178, 14, 147, 115, 110, 124, 228, 160, 9, 1,
                130, 199, 252, 198, 46, 123, 230, 129, 186, 16, 161, 138, 134, 213,
            ],
            hash_v3: [
                0, 71, 118, 49, 62, 12, 47, 80, 47, 195, 132, 140, 234, 68, 223, 69, 171, 154, 13,
                141, 229, 89, 195, 99, 212, 91, 135, 10, 143, 61, 201, 84,
            ],
        },
        TestCase {
            name: "02_max_amount_initialized_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: u64::MAX,
                delegate: None,
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                19, 76, 212, 251, 202, 235, 205, 25, 251, 143, 183, 128, 144, 54, 126, 210, 75,
                205, 2, 186, 84, 239, 39, 98, 157, 52, 238, 243, 226, 105, 40, 24,
            ],
            hash_v1: [
                19, 76, 212, 251, 202, 235, 205, 25, 251, 143, 183, 128, 144, 54, 126, 210, 75,
                205, 2, 186, 84, 239, 39, 98, 157, 52, 238, 243, 226, 105, 40, 24,
            ],
            hash_v3: [
                0, 88, 148, 2, 121, 104, 214, 193, 191, 30, 205, 152, 48, 189, 190, 96, 67, 180,
                120, 209, 233, 229, 232, 1, 72, 13, 222, 128, 80, 166, 90, 5,
            ],
        },
        TestCase {
            name: "03_max_amount_frozen_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: u64::MAX,
                delegate: None,
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                10, 186, 222, 252, 104, 38, 71, 142, 203, 234, 21, 59, 155, 69, 58, 148, 211, 230,
                44, 187, 121, 245, 2, 79, 5, 28, 111, 88, 198, 67, 37, 126,
            ],
            hash_v1: [
                10, 186, 222, 252, 104, 38, 71, 142, 203, 234, 21, 59, 155, 69, 58, 148, 211, 230,
                44, 187, 121, 245, 2, 79, 5, 28, 111, 88, 198, 67, 37, 126,
            ],
            hash_v3: [
                0, 84, 6, 235, 164, 19, 213, 196, 166, 141, 58, 94, 228, 71, 9, 173, 238, 67, 91,
                28, 116, 143, 166, 18, 148, 120, 7, 227, 91, 115, 30, 94,
            ],
        },
        TestCase {
            name: "04_max_amount_frozen_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: u64::MAX,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                34, 12, 230, 249, 174, 143, 161, 158, 75, 206, 86, 45, 253, 52, 148, 203, 125, 62,
                196, 210, 7, 216, 198, 80, 220, 251, 187, 46, 92, 161, 81, 55,
            ],
            hash_v1: [
                34, 12, 230, 249, 174, 143, 161, 158, 75, 206, 86, 45, 253, 52, 148, 203, 125, 62,
                196, 210, 7, 216, 198, 80, 220, 251, 187, 46, 92, 161, 81, 55,
            ],
            hash_v3: [
                0, 199, 77, 127, 145, 102, 130, 80, 140, 109, 54, 121, 33, 203, 155, 7, 56, 3, 175,
                233, 215, 82, 125, 186, 50, 71, 88, 71, 39, 207, 53, 150,
            ],
        },
        TestCase {
            name: "05_zero_amount_initialized_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                3, 116, 17, 236, 87, 92, 102, 163, 152, 61, 182, 33, 35, 206, 176, 64, 119, 66,
                233, 158, 86, 205, 18, 235, 148, 139, 7, 233, 146, 76, 214, 51,
            ],
            hash_v1: [
                3, 116, 17, 236, 87, 92, 102, 163, 152, 61, 182, 33, 35, 206, 176, 64, 119, 66,
                233, 158, 86, 205, 18, 235, 148, 139, 7, 233, 146, 76, 214, 51,
            ],
            hash_v3: [
                0, 137, 198, 218, 94, 228, 160, 58, 64, 52, 189, 15, 238, 17, 45, 174, 118, 138,
                243, 14, 158, 116, 0, 137, 8, 175, 111, 67, 97, 222, 234, 87,
            ],
        },
        TestCase {
            name: "06_zero_amount_initialized_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: None,
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                14, 51, 145, 21, 242, 240, 211, 203, 94, 227, 174, 67, 54, 120, 222, 119, 167, 193,
                3, 11, 172, 253, 212, 195, 91, 210, 110, 44, 75, 115, 23, 242,
            ],
            hash_v1: [
                14, 51, 145, 21, 242, 240, 211, 203, 94, 227, 174, 67, 54, 120, 222, 119, 167, 193,
                3, 11, 172, 253, 212, 195, 91, 210, 110, 44, 75, 115, 23, 242,
            ],
            hash_v3: [
                0, 94, 33, 255, 121, 48, 221, 196, 212, 122, 237, 143, 69, 185, 173, 112, 158, 9,
                187, 224, 54, 196, 11, 62, 82, 226, 232, 188, 253, 247, 188, 39,
            ],
        },
        TestCase {
            name: "07_zero_amount_frozen_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: None,
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                36, 29, 44, 77, 107, 65, 253, 11, 221, 150, 37, 14, 159, 144, 13, 63, 205, 180,
                214, 234, 144, 63, 201, 212, 251, 10, 237, 248, 118, 177, 174, 16,
            ],
            hash_v1: [
                36, 29, 44, 77, 107, 65, 253, 11, 221, 150, 37, 14, 159, 144, 13, 63, 205, 180,
                214, 234, 144, 63, 201, 212, 251, 10, 237, 248, 118, 177, 174, 16,
            ],
            hash_v3: [
                0, 239, 8, 205, 22, 245, 142, 219, 157, 28, 105, 55, 4, 196, 183, 0, 195, 210, 175,
                170, 96, 247, 25, 39, 96, 217, 255, 174, 30, 164, 87, 20,
            ],
        },
        TestCase {
            name: "08_zero_amount_frozen_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                9, 204, 52, 37, 54, 111, 219, 49, 154, 4, 11, 47, 102, 127, 14, 88, 87, 171, 32,
                64, 164, 119, 158, 167, 246, 103, 227, 215, 117, 151, 83, 223,
            ],
            hash_v1: [
                9, 204, 52, 37, 54, 111, 219, 49, 154, 4, 11, 47, 102, 127, 14, 88, 87, 171, 32,
                64, 164, 119, 158, 167, 246, 103, 227, 215, 117, 151, 83, 223,
            ],
            hash_v3: [
                0, 49, 23, 225, 160, 118, 218, 19, 71, 223, 185, 97, 106, 2, 252, 69, 158, 37, 117,
                64, 118, 76, 102, 191, 5, 202, 231, 132, 106, 124, 232, 207,
            ],
        },
        TestCase {
            name: "09_one_token_initialized_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 1,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                46, 50, 112, 80, 36, 45, 175, 148, 110, 194, 122, 122, 185, 78, 130, 155, 97, 209,
                62, 77, 27, 142, 164, 202, 71, 199, 246, 165, 99, 120, 19, 176,
            ],
            hash_v1: [
                11, 241, 4, 224, 23, 166, 206, 6, 127, 136, 22, 186, 182, 113, 70, 101, 177, 94,
                124, 59, 118, 196, 68, 78, 83, 40, 162, 33, 75, 58, 255, 113,
            ],
            hash_v3: [
                0, 63, 225, 214, 158, 134, 62, 4, 135, 117, 42, 163, 102, 116, 41, 216, 124, 212,
                35, 103, 48, 77, 228, 22, 102, 102, 151, 64, 0, 10, 48, 42,
            ],
        },
        TestCase {
            name: "10_one_token_initialized_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 1,
                delegate: None,
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            },
            hash_v2: [
                18, 206, 161, 180, 135, 26, 106, 117, 6, 186, 79, 252, 218, 204, 107, 210, 220,
                195, 156, 18, 253, 88, 116, 73, 175, 243, 105, 68, 107, 179, 248, 102,
            ],
            hash_v1: [
                25, 186, 144, 156, 125, 141, 31, 115, 197, 23, 74, 135, 232, 212, 217, 210, 55, 37,
                186, 157, 215, 61, 60, 61, 115, 15, 145, 62, 85, 172, 55, 91,
            ],
            hash_v3: [
                0, 6, 12, 92, 45, 41, 248, 100, 65, 189, 93, 93, 173, 145, 129, 1, 231, 109, 67,
                57, 20, 250, 94, 14, 52, 174, 8, 100, 137, 109, 234, 171,
            ],
        },
        TestCase {
            name: "11_one_token_frozen_no_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 1,
                delegate: None,
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                27, 199, 210, 205, 85, 105, 40, 209, 146, 151, 75, 194, 168, 252, 232, 53, 105, 37,
                29, 165, 23, 81, 137, 68, 226, 201, 11, 153, 37, 97, 80, 221,
            ],
            hash_v1: [
                29, 42, 213, 250, 142, 168, 199, 109, 174, 208, 208, 158, 178, 244, 46, 201, 202,
                154, 45, 122, 40, 119, 69, 77, 107, 69, 136, 252, 205, 14, 192, 196,
            ],
            hash_v3: [
                0, 151, 149, 226, 219, 123, 239, 106, 69, 158, 10, 108, 196, 64, 252, 208, 179,
                139, 205, 11, 212, 128, 234, 130, 211, 182, 37, 125, 47, 22, 120, 69,
            ],
        },
        TestCase {
            name: "12_one_token_frozen_with_delegate".to_string(),
            token_data: TokenData {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 1,
                delegate: Some(delegate_pubkey),
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            },
            hash_v2: [
                1, 5, 33, 95, 42, 18, 97, 191, 50, 98, 195, 200, 222, 175, 82, 108, 101, 215, 99,
                5, 56, 246, 37, 2, 239, 222, 165, 54, 224, 14, 79, 140,
            ],
            hash_v1: [
                16, 77, 96, 167, 224, 109, 158, 165, 126, 19, 194, 59, 207, 7, 179, 74, 214, 31,
                66, 244, 91, 19, 210, 225, 191, 3, 253, 81, 86, 68, 134, 184,
            ],
            hash_v3: [
                0, 58, 231, 217, 156, 143, 64, 49, 230, 235, 97, 185, 105, 13, 178, 198, 72, 143,
                251, 68, 165, 199, 215, 164, 116, 86, 156, 15, 150, 65, 149, 60,
            ],
        },
    ];
    for test_case in test_cases.iter() {
        assert_eq!(test_case.token_data.hash_v1().unwrap(), test_case.hash_v1);
        assert_eq!(test_case.token_data.hash_v2().unwrap(), test_case.hash_v2);
        assert_eq!(
            test_case.token_data.hash_sha_flat().unwrap(),
            test_case.hash_v3
        );
    }
}
