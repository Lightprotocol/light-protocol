use anchor_lang::prelude::*;

#[error_code]
pub enum MerkleTreeError {
    #[msg("Invalid height, it has to be greater than 0")]
    HeightZero,
    #[msg("Invalid height, it cannot exceed the maximum allowed height")]
    HeightHigherThanMax,
    #[msg("Invalid number of inputs.")]
    PoseidonInvalidNumberOfInputs,
    #[msg("Input is an empty slice.")]
    PoseidonEmptyInput,
    #[msg("Invalid length of the input.")]
    PoseidonInvalidInputLength,
    #[msg("Failed to convert bytes into a prime field element.")]
    PoseidonBytesToPrimeFieldElement,
    #[msg("Input is larger than the modulus of the prime field.")]
    PoseidonInputLargerThanModulus,
    #[msg("Failed to convert a vector of bytes into an array.")]
    PoseidonVecToArray,
    #[msg("Failed to convert the number of inputs from u64 to u8.")]
    PoseidonU64Tou8,
    #[msg("Failed to convert bytes to BigInt")]
    PoseidonBytesToBigInt,
    #[msg("Invalid width. Choose a width between 2 and 16 for 1 to 15 inputs.")]
    PoseidonInvalidWidthCircom,
    #[msg("Unknown Poseidon syscall error")]
    PoseidonUnknown,
}
