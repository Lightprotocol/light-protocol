use thiserror::Error;

#[derive(Debug, Error)]
pub enum HasherError {
    #[error("Invalid height, it has to be greater than 0")]
    HeightZero,
    #[error("Invalid height, it cannot exceed the maximum allowed height")]
    HeightHigherThanMax,
    #[error("Invalid number of roots, it has to be greater than 0")]
    RootsZero,
    #[error("Invalid root index, it exceeds the root buffer size")]
    RootHigherThanMax,
    #[error("Merkle tree is full, cannot append more leaves.")]
    TreeFull,
    #[error("Provided proof is larger than the height of the tree.")]
    ProofTooLarge,
    #[error("Invalid Merkle proof, stopping the update operation.")]
    InvalidProof,
    #[error("Attempting to update the leaf which was updated by an another newest change.")]
    CannotUpdateLeaf,
    #[error("Cannot update tree without changelog, only `append` is supported.")]
    AppendOnly,
    #[error("Invalid index, it exceeds the number of elements.")]
    IndexHigherThanMax,
    #[error("Could not find the low element.")]
    LowElementNotFound,
    #[error("Low element is greater or equal to the provided new element.")]
    LowElementGreaterOrEqualToNewElement,
    #[error("The provided new element is greater or equal to the next element.")]
    NewElementGreaterOrEqualToNextElement,
    #[error("Integer overflow, value too large")]
    IntegerOverflow,
    #[error("Invalid number of inputs.")]
    PoseidonInvalidNumberOfInputs,
    #[error("Input is an empty slice.")]
    PoseidonEmptyInput,
    #[error("Invalid length of the input.")]
    PoseidonInvalidInputLength,
    #[error("Failed to convert bytes into a prime field element.")]
    PoseidonBytesToPrimeFieldElement,
    #[error("Input is larger than the modulus of the prime field.")]
    PoseidonInputLargerThanModulus,
    #[error("Failed to convert a vector of bytes into an array.")]
    PoseidonVecToArray,
    #[error("Failed to convert the number of inputs from u64 to u8.")]
    PoseidonU64Tou8,
    #[error("Failed to convert bytes to BigInt")]
    PoseidonBytesToBigInt,
    #[error("Invalid width. Choose a width between 2 and 16 for 1 to 15 inputs.")]
    PoseidonInvalidWidthCircom,
    #[error("Unknown Poseidon syscall error")]
    PoseidonUnknown,
}
