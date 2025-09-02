use std::time::Duration;

use forester_utils::rpc_pool::PoolError;
use light_client::rpc::errors::RpcError;
use light_compressed_account::TreeType;
use light_registry::errors::RegistryError;
use photon_api::apis::{default_api::GetCompressedAccountProofPostError, Error as PhotonApiError};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum ForesterError {
    #[error("Element is not eligible for foresting")]
    NotEligible,

    #[error("Registration error: {0}")]
    Registration(#[from] RegistrationError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),

    #[error("Work report error: {0}")]
    WorkReport(#[from] WorkReportError),

    #[error("Epoch registration returned no result")]
    EmptyRegistration,

    #[error("Failed to register epoch {epoch}: {error}")]
    RegistrationFailed { epoch: u64, error: String },

    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),

    #[error("RPC pool error: {0}")]
    RpcPool(#[from] PoolError),

    #[error("Program error: {0}")]
    Program(#[from] ProgramError),

    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    #[error("Subscription error: {0}")]
    Subscription(String),

    #[error("Initialization error: {0}")]
    Initialization(#[from] InitializationError),

    #[error("Account deserialization error: {0}")]
    AccountDeserialization(#[from] AccountDeserializationError),

    #[error("Invalid tree type: {0}")]
    InvalidTreeType(TreeType),

    #[error("Not in active phase")]
    NotInActivePhase,

    #[error("Forester error: {error}")]
    General { error: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum RegistrationError {
    #[error("Too late to register for epoch {epoch}. Current slot: {current_slot}, Registration end: {registration_end}")]
    RegistrationPhaseEnded {
        epoch: u64,
        current_slot: u64,
        registration_end: u64,
    },

    #[error("Epoch registration returned no result")]
    EmptyRegistration,

    #[error("Failed to register epoch {epoch}: {error}")]
    RegistrationFailed { epoch: u64, error: String },

    #[error("Failed to register for epoch {epoch} after {attempts} attempts")]
    MaxRetriesExceeded { epoch: u64, attempts: u32 },

    #[error("Failed to register forester: {0}")]
    ForesterRegistration(String),

    #[error("ForesterEpochPda not found for address {pda_address}")]
    ForesterEpochPdaNotFound { epoch: u64, pda_address: Pubkey },

    #[error("Failed to fetch ForesterEpochPda for address {pda_address}: {error}")]
    ForesterEpochPdaFetchFailed { pda_address: Pubkey, error: String },

    #[error("EpochPda not found for address {pda_address}")]
    EpochPdaNotFound { epoch: u64, pda_address: Pubkey },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid keypair data: {0}")]
    InvalidKeypair(String),

    #[error("Invalid pubkey: {field} - {error}")]
    InvalidPubkey { field: &'static str, error: String },

    #[error("Invalid derivation: {reason}")]
    InvalidDerivation { reason: String },

    #[error("JSON parsing error: {field} - {error}")]
    JsonParse { field: &'static str, error: String },
}

#[derive(Error, Debug)]
pub enum AccountDeserializationError {
    #[error("Failed to deserialize batch state tree account: {error}")]
    BatchStateMerkleTree { error: String },

    #[error("Failed to deserialize batch address tree account: {error}")]
    BatchAddressMerkleTree { error: String },
}

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Indexer error: {error}")]
    General { error: String },
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Failed to send work report for epoch {epoch}: {error}")]
    WorkReportSend { epoch: u64, error: String },

    #[error("Channel error: {error}")]
    General { error: String },
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Slot length overflow: value {value} cannot fit in u32")]
    SlotLengthOverflow { value: u64 },

    #[error(
        "Timeout calculation overflow: slot_duration {slot_duration:?} * slot_length {slot_length}"
    )]
    TimeoutCalculationOverflow {
        slot_duration: Duration,
        slot_length: u32,
    },
}

#[derive(Error, Debug)]
pub enum InitializationError {
    #[error("Failed to start forester after {attempts} attempts. Last error: {error}")]
    MaxRetriesExceeded { attempts: u32, error: String },

    #[error("Unexpected initialization error: {0}")]
    Unexpected(String),
}

#[derive(Error, Debug)]
pub enum WorkReportError {
    #[error("Not in report work phase for epoch {epoch}")]
    NotInReportPhase { epoch: u64 },

    #[error("Work already reported for epoch {epoch}")]
    AlreadyReported { epoch: u64 },

    #[error("Registry instruction error: {error_code}")]
    RegistryInstruction { error_code: u32 },

    #[error("Transaction failed: {0}")]
    Transaction(#[from] Box<RpcError>),
}

impl WorkReportError {
    pub(crate) fn from_registry_error(error_code: u32, epoch: u64) -> Result<(), Self> {
        match error_code {
            code if code == RegistryError::ForesterAlreadyReportedWork as u32 => {
                info!("Work already reported for epoch {}. Skipping.", epoch);
                Ok(())
            }
            code if code == RegistryError::NotInReportWorkPhase as u32 => {
                warn!("Not in report work phase for epoch {}. Skipping.", epoch);
                Ok(())
            }
            code => Err(Self::RegistryInstruction { error_code: code }),
        }
    }
}

#[derive(Error, Debug)]
pub enum PhotonApiErrorWrapper {
    #[error(transparent)]
    GetCompressedAccountProofPostError(#[from] PhotonApiError<GetCompressedAccountProofPostError>),
}
impl ForesterError {
    pub fn indexer<E: std::fmt::Display>(error: E) -> Self {
        Self::Indexer(IndexerError::General {
            error: error.to_string(),
        })
    }

    pub fn channel<E: std::fmt::Display>(error: E) -> Self {
        Self::Channel(ChannelError::General {
            error: error.to_string(),
        })
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ForesterError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::channel(err)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for ForesterError {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::channel(err)
    }
}

impl From<tokio::task::JoinError> for ForesterError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Other(err.into())
    }
}
