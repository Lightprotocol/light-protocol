use groth16_solana::errors::Groth16Error;
use light_compressed_account::CompressedAccountError;
use light_hasher::HasherError;
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

    #[error("Groth16-Solana Error: {0}")]
    Groth16SolanaError(Groth16Error),

    #[error("Cannot change endianness")]
    ChangeEndiannessError,

    #[error("Cannot parse inputs")]
    InputsParsingError,

    #[error("Wrong number of UTXO's")]
    WrongNumberOfUtxos,
    #[error("Compressed account error: {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
}

impl From<Groth16Error> for ProverClientError {
    fn from(error: Groth16Error) -> Self {
        ProverClientError::Groth16SolanaError(error)
    }
}

impl From<HasherError> for ProverClientError {
    fn from(error: HasherError) -> Self {
        ProverClientError::GenericError(format!("Hasher error: {:?}", error))
    }
}
