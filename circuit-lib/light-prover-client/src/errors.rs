use groth16_solana::errors::Groth16Error;
use light_utils::UtilsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProverClientError {
    #[error("RPC error")]
    RpcError,
    #[error("Error: {0}")]
    GenericError(String),

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
    #[error("Utils error: {0}")]
    UtilsError(#[from] UtilsError),
}

impl From<Groth16Error> for ProverClientError {
    fn from(error: Groth16Error) -> Self {
        ProverClientError::Groth16SolanaError(error)
    }
}
