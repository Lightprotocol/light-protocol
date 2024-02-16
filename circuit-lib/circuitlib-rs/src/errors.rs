use ark_relations::r1cs::SynthesisError;
use ark_serialize::SerializationError;
use color_eyre::Report;
use groth16_solana::errors::Groth16Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CircuitsError {
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
}

impl From<SerializationError> for CircuitsError {
    fn from(error: SerializationError) -> Self {
        CircuitsError::ArkworksSerializationError(error.to_string())
    }
}

impl From<SynthesisError> for CircuitsError {
    fn from(error: SynthesisError) -> Self {
        CircuitsError::ArkworksProverError(error.to_string())
    }
}

impl From<Report> for CircuitsError {
    fn from(error: Report) -> Self {
        CircuitsError::GenericError(error.to_string())
    }
}

impl From<Groth16Error> for CircuitsError {
    fn from(error: Groth16Error) -> Self {
        CircuitsError::Groth16SolanaError(error)
    }
}
