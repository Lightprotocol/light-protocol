use thiserror::Error;

#[derive(Error, Debug)]
pub enum ForesterUtilsError {
    #[error("parse error: {0:?}")]
    Parse(String),
    #[error("prover error: {0:?}")]
    Prover(String),
    #[error("rpc error: {0:?}")]
    Rpc(String),
    #[error("indexer error: {0:?}")]
    Indexer(String),
    #[error("invalid slot number")]
    InvalidSlotNumber,
}
