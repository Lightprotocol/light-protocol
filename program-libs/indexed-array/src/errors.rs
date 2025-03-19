use light_hasher::HasherError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum IndexedArrayError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid index, it exceeds the number of elements.")]
    IndexHigherThanMax,
    #[error("Could not find the low element.")]
    LowElementNotFound,
    #[error("Low element is greater or equal to the provided new element.")]
    LowElementGreaterOrEqualToNewElement,
    #[error("The provided new element is greater or equal to the next element.")]
    NewElementGreaterOrEqualToNextElement,
    #[error("The element already exists, but was expected to be absent.")]
    ElementAlreadyExists,
    #[error("The element does not exist, but was expected to be present.")]
    ElementDoesNotExist,
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Indexed array is full, cannot append more elements")]
    ArrayFull,
}
