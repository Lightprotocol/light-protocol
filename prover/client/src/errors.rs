use light_hasher::HasherError;
use solana_bn254::compression::AltBn128CompressionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProverClientError {
    #[error("RPC error")]
    RpcError,

    #[error("Error: {0}")]
    GenericError(String),

    #[error("Prover server error: {0}")]
    ProverServerError(String),

    #[error("Arkworks prover error: {0}")]
    ArkworksProverError(String),

    #[error("Arkworks serialization error: {0}")]
    ArkworksSerializationError(String),

    #[error("Cannot change endianness")]
    ChangeEndiannessError,

    #[error("Cannot parse inputs")]
    InputsParsingError,

    #[error("Wrong number of UTXO's")]
    WrongNumberOfUtxos,

    #[error("AltBn128Error error: {0}")]
    AltBn128CompressionError(String),
}

impl From<AltBn128CompressionError> for ProverClientError {
    fn from(error: AltBn128CompressionError) -> Self {
        ProverClientError::AltBn128CompressionError(error.to_string())
    }
}

impl From<HasherError> for ProverClientError {
    fn from(error: HasherError) -> Self {
        ProverClientError::GenericError(format!("Hasher error: {:?}", error))
    }
}
