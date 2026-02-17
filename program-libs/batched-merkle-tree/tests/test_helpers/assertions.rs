use std::fmt::Debug;

use light_batched_merkle_tree::errors::BatchedMerkleTreeError;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_zero_copy::errors::ZeroCopyError;

/// Assert that a result is an error and matches the expected error.
pub fn assert_error<T, E>(result: Result<T, E>, expected: E, context: &str)
where
    T: Debug,
    E: Debug + PartialEq,
{
    match result {
        Ok(val) => panic!(
            "{}: Expected error {:?}, but got Ok({:?})",
            context, expected, val
        ),
        Err(actual) => assert_eq!(
            actual, expected,
            "{}: Error mismatch. Expected {:?}, got {:?}",
            context, expected, actual
        ),
    }
}

/// Assert that a result is a ZeroCopyError.
pub fn assert_zerocopy_error<T>(result: Result<T, BatchedMerkleTreeError>, context: &str)
where
    T: Debug,
{
    match result {
        Ok(val) => panic!("{}: Expected ZeroCopyError, but got Ok({:?})", context, val),
        Err(BatchedMerkleTreeError::ZeroCopy(_)) => {
            // Success - it's a ZeroCopy error
        }
        Err(other) => panic!("{}: Expected ZeroCopyError, but got {:?}", context, other),
    }
}

/// Assert that a result is a MerkleTreeMetadataError with the specific type.
pub fn assert_metadata_error<T>(
    result: Result<T, BatchedMerkleTreeError>,
    expected: MerkleTreeMetadataError,
    context: &str,
) where
    T: Debug,
{
    match result {
        Ok(val) => panic!(
            "{}: Expected MerkleTreeMetadataError::{:?}, but got Ok({:?})",
            context, expected, val
        ),
        Err(BatchedMerkleTreeError::MerkleTreeMetadata(actual)) => {
            assert_eq!(
                actual, expected,
                "{}: MerkleTreeMetadataError mismatch. Expected {:?}, got {:?}",
                context, expected, actual
            );
        }
        Err(other) => panic!(
            "{}: Expected MerkleTreeMetadataError::{:?}, but got {:?}",
            context, expected, other
        ),
    }
}

/// Assert that a result is an AccountError.
pub fn assert_account_error<T>(result: Result<T, BatchedMerkleTreeError>, context: &str)
where
    T: Debug,
{
    match result {
        Ok(val) => panic!("{}: Expected AccountError, but got Ok({:?})", context, val),
        Err(BatchedMerkleTreeError::AccountError(_)) => {
            // Success - it's an AccountError
        }
        Err(other) => panic!("{}: Expected AccountError, but got {:?}", context, other),
    }
}

#[cfg(test)]
mod tests {
    use light_account_checks::error::AccountError;

    use super::*;

    #[test]
    fn test_assert_error_catches_mismatch() {
        let result: Result<(), BatchedMerkleTreeError> = Err(BatchedMerkleTreeError::InvalidIndex);
        assert_error(result, BatchedMerkleTreeError::InvalidIndex, "Should match");
    }

    #[test]
    fn test_assert_zerocopy_error() {
        let result: Result<(), BatchedMerkleTreeError> =
            Err(BatchedMerkleTreeError::ZeroCopy(ZeroCopyError::Size));
        assert_zerocopy_error(result, "Should be ZeroCopy error");
    }

    #[test]
    fn test_assert_metadata_error() {
        let result: Result<(), BatchedMerkleTreeError> = Err(
            BatchedMerkleTreeError::MerkleTreeMetadata(MerkleTreeMetadataError::InvalidTreeType),
        );
        assert_metadata_error(
            result,
            MerkleTreeMetadataError::InvalidTreeType,
            "Should match InvalidTreeType",
        );
    }

    #[test]
    fn test_assert_account_error() {
        let result: Result<(), BatchedMerkleTreeError> = Err(BatchedMerkleTreeError::AccountError(
            AccountError::InvalidDiscriminator,
        ));
        assert_account_error(result, "Should be AccountError");
    }
}
