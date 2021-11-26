use crate::parse_verifyingkey_254::*;
use crate::verifyingkey_254_hc::*;
use ark_ff::{Fp256, FromBytes};
use ark_groth16::{prepare_inputs, prepare_verifying_key};
use ark_std::{One, Zero};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryInto;

use crate::pi_254_instructions::*;
use crate::pi_254_ranges::*;

#[test]
fn pi_254_test_with_7_inputs() {
    let ix_order_array_mock: [u8; 1809] = [
        40, 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 46, 41, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
        43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
        44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 46, 41, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 46, 41, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
        56, 56, 46, 41, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
        57, 57, 57, 57, 57, 57, 57, 46, 41, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
        58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 46, 47, 48,
    ];

    let current_index_mock: [u8; 1809] = [
        40, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 41, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
        92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
        112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
        130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
        184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
        220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
        238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        46, 47, 48,
    ];

    // 7 inputs á 32 bytes. For bn254 curve. Skip the first two bytes.
    let inputs_bytes = [
        0, 0, 139, 101, 98, 198, 106, 26, 157, 253, 217, 85, 208, 20, 62, 194, 7, 229, 230, 196,
        195, 91, 112, 106, 227, 5, 89, 90, 68, 176, 218, 172, 23, 34, 1, 0, 63, 128, 161, 110, 190,
        67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49,
        225, 114, 78, 100, 48, 224, 137, 70, 92, 255, 138, 142, 119, 60, 162, 100, 218, 34, 199,
        20, 246, 167, 35, 235, 134, 225, 54, 67, 209, 246, 194, 128, 223, 27, 115, 112, 25, 13,
        113, 159, 110, 133, 81, 26, 27, 23, 26, 184, 1, 175, 109, 99, 85, 188, 45, 119, 213, 233,
        137, 186, 52, 25, 2, 52, 160, 2, 122, 107, 18, 62, 183, 110, 221, 22, 145, 254, 220, 22,
        239, 208, 169, 202, 190, 70, 169, 206, 157, 185, 145, 226, 81, 196, 182, 29, 125, 181, 119,
        242, 71, 107, 10, 167, 4, 10, 212, 160, 90, 85, 209, 147, 16, 119, 99, 254, 93, 143, 137,
        91, 121, 198, 246, 245, 79, 190, 201, 63, 229, 250, 134, 157, 180, 3, 12, 228, 236, 174,
        112, 138, 244, 188, 161, 144, 60, 210, 99, 115, 64, 69, 63, 35, 176, 250, 189, 20, 28, 23,
        2, 19, 94, 196, 88, 14, 51, 12, 21,
    ];

    // TODO: currently switching types from fq to fr. double check this before production.
    let input1 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[2..34]).unwrap();
    let input2 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[34..66]).unwrap();
    let input3 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[66..98]).unwrap();
    let input4 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[98..130]).unwrap();
    let input5 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[130..162]).unwrap();
    let input6 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[162..194]).unwrap();

    let input7 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[194..226]).unwrap();

    let inputs: Vec<Fp256<ark_bn254::FrParameters>> =
        vec![input1, input2, input3, input4, input5, input6, input7];
    // parse vk and prepare it.
    // prepare inputs with pvk and inputs for bn254.
    let vk = get_pvk_from_bytes_254().unwrap();
    let pvk = prepare_verifying_key(&vk);
    let prepared_inputs = prepare_inputs(&pvk, &inputs);
    println!("prepared inputs from library: {:?}", prepared_inputs);

    // execute full onchain mock -> same results?
    // call processor with i_order
    // need local account struct those pass along and r/w to
    // mocking the parsing of instruction_data between 42-45 and 56,57,58  (current_index)

    // init local bytes array (mocking onchain account)
    let mock_account = [0; 4972];
    // ix_order_array
    // for each ix call processor. If applicable with extra instruction_data
    // let mut current_index = 99;
    let mut account_data = PiTestBytes::unpack(&mock_account).unwrap();

    for index in 0..1809 {
        println!("c ixorderarr: {}", ix_order_array_mock[index]);
        println!("index: {:?}", index);
        test_pi_254_process_instruction(
            // ix_order_array_mock[usize::from(index)],
            ix_order_array_mock[index],
            &mut account_data,
            &inputs,
            usize::from(current_index_mock[index]), // usize::from(current_index_mock[usize::from(index)]),
        );
    }

    // let prepared_inputs_cus = prepare_inputs_custom(r, &inputs).unwrap(); // CUSTOM
    // 1) does own implementation work? => compare to lib call
    // // for custom
    // let mut pvk_values = vec![
    //     get_gamma_abc_g1_0(),
    //     get_gamma_abc_g1_1(),
    //     get_gamma_abc_g1_2(),
    //     get_gamma_abc_g1_3(),
    //     get_gamma_abc_g1_4(),
    // ];

    // call the implementation function with inputs
    // assert_eq!(false, true);
    // 2) does own implementation yield same results as "onchain" implementation (split)?
    // test_pi_254_onchain_mock(inputs)
    // let a = Fp256::<ark_ed_on_bn254::FqParameters>::one();
    // println!("a: {:?}", a);
}

fn test_pi_254_onchain_mock(inputs: Vec<ark_ff::Fp256<ark_bn254::FrParameters>>) {

    // first, compare the whole thing with the lib function

    // if fails, go the offchain impl route. (step by step)

    // rebuild ix
    // 1by1 compare:
    // processor call => build processor for the new thing as well
    // account is local only ofc
    // currentIndex increment
    // -> hardcode "1 loop" off implementation ->

    // call assert_wrap in loops with increment
}

fn test_pi_254_process_instruction(
    id: u8,
    account: &mut PiTestBytes,
    public_inputs: &[ark_ff::Fp256<ark_ed_on_bn254::FqParameters>],
    current_index: usize,
) {
    // i_order: [0,1,256*2,6,    1,256*3,6, .... x4]
    if id == 40 {
        init_pairs_instruction(
            &public_inputs,
            &mut account.i_1_range,
            &mut account.x_1_range,
            &mut account.i_2_range,
            &mut account.x_2_range,
            &mut account.i_3_range,
            &mut account.x_3_range,
            &mut account.i_4_range,
            &mut account.x_4_range,
            &mut account.i_5_range,
            &mut account.x_5_range,
            &mut account.i_6_range,
            &mut account.x_6_range,
            &mut account.i_7_range,
            &mut account.x_7_range,
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
        );

        let indices = [
            I_1_RANGE_INDEX,
            X_1_RANGE_INDEX,
            I_2_RANGE_INDEX,
            X_2_RANGE_INDEX,
            I_3_RANGE_INDEX,
            X_3_RANGE_INDEX,
            I_4_RANGE_INDEX,
            X_4_RANGE_INDEX,
            I_5_RANGE_INDEX,
            X_5_RANGE_INDEX,
            I_6_RANGE_INDEX,
            X_6_RANGE_INDEX,
            I_7_RANGE_INDEX,
            X_7_RANGE_INDEX,
            G_IC_X_RANGE_INDEX,
            G_IC_Y_RANGE_INDEX,
            G_IC_Z_RANGE_INDEX,
        ];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 41 {
        init_res_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 42 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_1_range,
            &account.x_1_range,
            current_index,
        ); // 1 of 256
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 43 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_2_range,
            &account.x_2_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 44 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_3_range,
            &account.x_3_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 45 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_4_range,
            &account.x_4_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 46 {
        maths_g_ic_instruction(
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
            &account.res_x_range,
            &account.res_y_range,
            &account.res_z_range,
        );
        let indices = [G_IC_X_RANGE_INDEX, G_IC_Y_RANGE_INDEX, G_IC_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 47 {
        // migrated from preprocessor
        g_ic_into_affine_1(
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range, // only one changing
        );
        let indices = [G_IC_X_RANGE_INDEX, G_IC_Y_RANGE_INDEX, G_IC_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 48 {
        // migrated from preprocessor
        g_ic_into_affine_2(
            &account.g_ic_x_range,
            &account.g_ic_y_range,
            &account.g_ic_z_range,
            &mut account.x_1_range,
        );
        let indices = [X_1_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 56 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_5_range,
            &account.x_5_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 57 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_6_range,
            &account.x_6_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    } else if id == 58 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_7_range,
            &account.x_7_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices {
            account.changed_variables[i] = true;
        }
    }
}

#[derive(Clone)]
pub struct PiTestBytes {
    is_initialized: bool,
    pub found_root: u8,
    pub found_nullifier: u8,
    pub executed_withdraw: u8,
    pub signing_address: Vec<u8>, // is relayer address
    pub relayer_refund: Vec<u8>,
    pub to_address: Vec<u8>,
    pub amount: Vec<u8>,
    pub nullifier_hash: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub data_hash: Vec<u8>,         // is commit hash until changed
    pub tx_integrity_hash: Vec<u8>, // is calculated on-chain from to_address, amount, signing_address,
    //root does not have to be saved for it is looked for immediately when added
    //adding 32 + 8 + 32 + 8 + 32 + 32 + 32 = 176
    //total added 3 + 176 = 179
    //memory variables
    pub i_1_range: Vec<u8>,
    pub x_1_range: Vec<u8>,
    pub i_2_range: Vec<u8>,
    pub x_2_range: Vec<u8>,
    pub i_3_range: Vec<u8>,
    pub x_3_range: Vec<u8>,
    pub i_4_range: Vec<u8>,
    pub x_4_range: Vec<u8>,
    // added 6 new ranges
    pub i_5_range: Vec<u8>,
    pub x_5_range: Vec<u8>,
    pub i_6_range: Vec<u8>,
    pub x_6_range: Vec<u8>,
    pub i_7_range: Vec<u8>,
    pub x_7_range: Vec<u8>,

    pub res_x_range: Vec<u8>,
    pub res_y_range: Vec<u8>,
    pub res_z_range: Vec<u8>,
    pub g_ic_x_range: Vec<u8>,
    pub g_ic_y_range: Vec<u8>,
    pub g_ic_z_range: Vec<u8>,
    pub current_instruction_index: usize,

    pub changed_variables: [bool; 20],
    pub changed_constants: [bool; 17],
}
impl Sealed for PiTestBytes {}
impl IsInitialized for PiTestBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for PiTestBytes {
    const LEN: usize = 4972; // 1020

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, PiTestBytes::LEN];

        let (
            is_initialized,
            found_root,
            found_nullifier,
            executed_withdraw,
            signing_address, // is relayer address
            relayer_refund,
            to_address,
            amount,
            nullifier_hash,
            root_hash,
            data_hash, // is commit hash until changed
            tx_integrity_hash,
            current_instruction_index,
            i_1_range, // 32b
            x_1_range, // 96b + constructor
            i_2_range,
            x_2_range,
            i_3_range,
            x_3_range,
            i_4_range,
            x_4_range,
            i_5_range,
            x_5_range,
            i_6_range,
            x_6_range,
            i_7_range,
            x_7_range,
            res_x_range,
            res_y_range,
            res_z_range,
            g_ic_x_range,
            g_ic_y_range,
            g_ic_z_range, // 144b 3*48 // actually now 3*32
            //until here 1020 bytes // actuall 288 more now so 1308 bytes
            unused_remainder,
        ) = array_refs![
            input, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 32, 64, 32, 64, 32, 64, 32, 64, 32,
            64, 32, 64, 32, 64, 32, 32, 32, 32, 32, 32,   //  48, 48, 48, 48, 48, 48, replaced
            3888 // 3792 was without the last 6 change down  // 3952 {128 less (1-4) and 288 more (5-7)}
        ];
        msg!("unpacked");

        Ok(PiTestBytes {
            is_initialized: true,

            found_root: found_root[0],                     //0
            found_nullifier: found_nullifier[0],           //1
            executed_withdraw: executed_withdraw[0],       //2
            signing_address: signing_address.to_vec(),     //3
            relayer_refund: relayer_refund.to_vec(),       //4
            to_address: to_address.to_vec(),               //5
            amount: amount.to_vec(),                       //6
            nullifier_hash: nullifier_hash.to_vec(),       //7
            root_hash: root_hash.to_vec(),                 //8
            data_hash: data_hash.to_vec(),                 //9
            tx_integrity_hash: tx_integrity_hash.to_vec(), //10

            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            i_1_range: i_1_range.to_vec(),       //0
            x_1_range: x_1_range.to_vec(),       //1
            i_2_range: i_2_range.to_vec(),       //2
            x_2_range: x_2_range.to_vec(),       //3
            i_3_range: i_3_range.to_vec(),       //4
            x_3_range: x_3_range.to_vec(),       //5
            i_4_range: i_4_range.to_vec(),       //6
            x_4_range: x_4_range.to_vec(),       //7
            i_5_range: i_5_range.to_vec(),       //8
            x_5_range: x_5_range.to_vec(),       //9
            i_6_range: i_6_range.to_vec(),       //10
            x_6_range: x_6_range.to_vec(),       //11
            i_7_range: i_7_range.to_vec(),       //12
            x_7_range: x_7_range.to_vec(),       //13
            res_x_range: res_x_range.to_vec(),   //14
            res_y_range: res_y_range.to_vec(),   //15
            res_z_range: res_z_range.to_vec(),   //16
            g_ic_x_range: g_ic_x_range.to_vec(), //17
            g_ic_y_range: g_ic_y_range.to_vec(), //18
            g_ic_z_range: g_ic_z_range.to_vec(), //19
            changed_variables: [false; 20],
            changed_constants: [false; 17],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, PiTestBytes::LEN];

        let (
            //constants
            is_initialized_dst,
            found_root_dst,
            found_nullifier_dst,
            executed_withdraw_dst,
            signing_address_dst, // is relayer address
            relayer_refund_dst,
            to_address_dst,
            amount_dst,
            nullifier_hash_dst,
            root_hash_dst,
            data_hash_dst,
            tx_integrity_hash_dst,
            //variables
            current_instruction_index_dst,
            //220
            i_1_range_dst,
            x_1_range_dst,
            i_2_range_dst,
            x_2_range_dst,
            i_3_range_dst,
            x_3_range_dst,
            i_4_range_dst,
            x_4_range_dst,
            i_5_range_dst,
            x_5_range_dst,
            i_6_range_dst,
            x_6_range_dst,
            i_7_range_dst,
            x_7_range_dst,
            res_x_range_dst,
            res_y_range_dst,
            res_z_range_dst,
            g_ic_x_range_dst,
            g_ic_y_range_dst,
            g_ic_z_range_dst,
            unused_remainder_dst,
        ) = mut_array_refs![
            dst, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 32, 64, 32, 64, 32, 64, 32, 64, 32,
            64, 32, 64, 32, 64, 32, 32, 32, 32, 32, 32, //  48, 48, 48, 48, 48, 48, replaced
            3888 // 3792 was without the last 6 change down  // 3952 {128 less (1-4) and 288 more (5-7)}
                  //dst, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 32, 96, 32, 96, 32, 96, 32, 96, 48,
                  //  48, 48, 48, 48, 48, 3952
        ];

        for (i, var_has_changed) in self.changed_variables.iter().enumerate() {
            if *var_has_changed {
                if i == 0 {
                    *i_1_range_dst = self.i_1_range.clone().try_into().unwrap();
                } else if i == 1 {
                    *x_1_range_dst = self.x_1_range.clone().try_into().unwrap();
                } else if i == 2 {
                    *i_2_range_dst = self.i_2_range.clone().try_into().unwrap();
                } else if i == 3 {
                    *x_2_range_dst = self.x_2_range.clone().try_into().unwrap();
                } else if i == 4 {
                    *i_3_range_dst = self.i_3_range.clone().try_into().unwrap();
                } else if i == 5 {
                    *x_3_range_dst = self.x_3_range.clone().try_into().unwrap();
                } else if i == 6 {
                    *i_4_range_dst = self.i_4_range.clone().try_into().unwrap();
                } else if i == 7 {
                    *x_4_range_dst = self.x_4_range.clone().try_into().unwrap();
                } else if i == 8 {
                    *i_5_range_dst = self.i_5_range.clone().try_into().unwrap();
                } else if i == 9 {
                    *x_5_range_dst = self.x_5_range.clone().try_into().unwrap();
                } else if i == 10 {
                    *i_6_range_dst = self.i_6_range.clone().try_into().unwrap();
                } else if i == 11 {
                    *x_6_range_dst = self.x_6_range.clone().try_into().unwrap();
                } else if i == 12 {
                    *i_7_range_dst = self.i_7_range.clone().try_into().unwrap();
                } else if i == 13 {
                    *x_7_range_dst = self.x_7_range.clone().try_into().unwrap();
                } else if i == 14 {
                    *res_x_range_dst = self.res_x_range.clone().try_into().unwrap();
                } else if i == 15 {
                    *res_y_range_dst = self.res_y_range.clone().try_into().unwrap();
                } else if i == 16 {
                    *res_z_range_dst = self.res_z_range.clone().try_into().unwrap();
                } else if i == 17 {
                    *g_ic_x_range_dst = self.g_ic_x_range.clone().try_into().unwrap();
                } else if i == 18 {
                    *g_ic_y_range_dst = self.g_ic_y_range.clone().try_into().unwrap();
                } else if i == 19 {
                    *g_ic_z_range_dst = self.g_ic_z_range.clone().try_into().unwrap();
                }
            } else {
                if i == 0 {
                    *i_1_range_dst = *i_1_range_dst;
                } else if i == 1 {
                    *x_1_range_dst = *x_1_range_dst;
                } else if i == 2 {
                    *i_2_range_dst = *i_2_range_dst;
                } else if i == 3 {
                    *x_2_range_dst = *x_2_range_dst;
                } else if i == 4 {
                    *i_3_range_dst = *i_3_range_dst;
                } else if i == 5 {
                    *x_3_range_dst = *x_3_range_dst;
                } else if i == 6 {
                    *i_4_range_dst = *i_4_range_dst;
                } else if i == 7 {
                    *x_4_range_dst = *x_4_range_dst;
                } else if i == 8 {
                    *i_5_range_dst = *i_5_range_dst;
                } else if i == 9 {
                    *x_5_range_dst = *x_5_range_dst;
                } else if i == 10 {
                    *i_6_range_dst = *i_6_range_dst;
                } else if i == 11 {
                    *x_6_range_dst = *x_6_range_dst;
                } else if i == 12 {
                    *i_7_range_dst = *i_7_range_dst;
                } else if i == 13 {
                    *x_7_range_dst = *x_7_range_dst;
                } else if i == 14 {
                    *res_x_range_dst = *res_x_range_dst;
                } else if i == 15 {
                    *res_y_range_dst = *res_y_range_dst;
                } else if i == 16 {
                    *res_z_range_dst = *res_z_range_dst;
                } else if i == 17 {
                    *g_ic_x_range_dst = *g_ic_x_range_dst;
                } else if i == 18 {
                    *g_ic_y_range_dst = *g_ic_y_range_dst;
                } else if i == 19 {
                    *g_ic_z_range_dst = *g_ic_z_range_dst;
                }
            };
        }

        for (i, const_has_changed) in self.changed_constants.iter().enumerate() {
            if *const_has_changed {
                if i == 0 {
                    *found_root_dst = [self.found_root.clone(); 1];
                } else if i == 1 {
                    *found_nullifier_dst = [self.found_nullifier.clone(); 1];
                } else if i == 2 {
                    *executed_withdraw_dst = [self.executed_withdraw.clone(); 1];
                } else if i == 3 {
                    *signing_address_dst = self.signing_address.clone().try_into().unwrap();
                } else if i == 4 {
                    *relayer_refund_dst = self.relayer_refund.clone().try_into().unwrap();
                } else if i == 5 {
                    *to_address_dst = self.to_address.clone().try_into().unwrap();
                } else if i == 6 {
                    *amount_dst = self.amount.clone().try_into().unwrap();
                } else if i == 7 {
                    *nullifier_hash_dst = self.nullifier_hash.clone().try_into().unwrap();
                } else if i == 8 {
                    *root_hash_dst = self.root_hash.clone().try_into().unwrap();
                } else if i == 9 {
                    *data_hash_dst = self.data_hash.clone().try_into().unwrap();
                } else if i == 10 {
                    *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
                }
            } else {
                if i == 0 {
                    *found_root_dst = *found_root_dst;
                } else if i == 1 {
                    *found_nullifier_dst = *found_nullifier_dst;
                } else if i == 2 {
                    *executed_withdraw_dst = *executed_withdraw_dst;
                } else if i == 3 {
                    *signing_address_dst = *signing_address_dst;
                } else if i == 4 {
                    *relayer_refund_dst = *relayer_refund_dst;
                } else if i == 5 {
                    *to_address_dst = *to_address_dst;
                } else if i == 6 {
                    *amount_dst = *amount_dst;
                } else if i == 7 {
                    *nullifier_hash_dst = *nullifier_hash_dst;
                } else if i == 8 {
                    *root_hash_dst = *root_hash_dst;
                } else if i == 9 {
                    *data_hash_dst = *data_hash_dst;
                } else if i == 10 {
                    *tx_integrity_hash_dst = *tx_integrity_hash_dst;
                }
            };
        }
        msg!("packed");
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *is_initialized_dst = [1u8; 1];
        *unused_remainder_dst = *unused_remainder_dst;
    }
}
