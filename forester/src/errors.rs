use std::time::Duration;

use anchor_lang::error::ERROR_CODE_OFFSET;
use forester_utils::rpc_pool::PoolError;
use light_client::rpc::errors::RpcError;
use light_compressed_account::TreeType;
use light_registry::errors::RegistryError;
use solana_program::{instruction::InstructionError, program_error::ProgramError, pubkey::Pubkey};
use solana_sdk::transaction::TransactionError;
use thiserror::Error;
use tracing::{info, warn};

use crate::{processor::v2::errors::V2Error, smart_transaction::SmartTransactionError};

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

    #[error(transparent)]
    V2(#[from] V2Error),

    #[error(transparent)]
    SmartTransaction(#[from] SmartTransactionError),

    #[error("Forester error: {error}")]
    General { error: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum RegistrationError {
    #[error("Too early to register for epoch {epoch}. Current slot: {current_slot}, Registration starts: {registration_start}")]
    RegistrationPhaseNotStarted {
        epoch: u64,
        current_slot: u64,
        registration_start: u64,
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

    #[error("JSON parsing error for {field}: {error}")]
    JsonParse { field: &'static str, error: String },

    #[error("Invalid {field}: {}", .invalid_values.join(", "))]
    InvalidArguments {
        field: &'static str,
        invalid_values: Vec<String>,
    },
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
            code if code == registry_error_code(RegistryError::ForesterAlreadyReportedWork) => {
                info!("Work already reported for epoch {}. Skipping.", epoch);
                Ok(())
            }
            code if code == registry_error_code(RegistryError::NotInReportWorkPhase) => {
                warn!("Not in report work phase for epoch {}. Skipping.", epoch);
                Ok(())
            }
            code => Err(Self::RegistryInstruction { error_code: code }),
        }
    }
}

impl ForesterError {
    pub fn is_not_in_active_phase(&self) -> bool {
        matches!(self, Self::NotInActivePhase)
    }

    pub fn is_forester_not_eligible(&self) -> bool {
        let forester_not_eligible_error_code =
            registry_error_code(RegistryError::ForesterNotEligible);

        match self {
            Self::NotEligible => true,
            Self::Rpc(rpc_error) => {
                rpc_custom_error_code(rpc_error) == Some(forester_not_eligible_error_code)
            }
            Self::SmartTransaction(smart_error) => {
                smart_error
                    .transaction_error()
                    .and_then(|error| match error {
                        TransactionError::InstructionError(
                            _,
                            InstructionError::Custom(error_code),
                        ) => Some(error_code),
                        _ => None,
                    })
                    == Some(forester_not_eligible_error_code)
            }
            Self::V2(v2_error) => {
                v2_error.custom_error_code() == Some(forester_not_eligible_error_code)
            }
            _ => false,
        }
    }

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

fn registry_error_code(error: RegistryError) -> u32 {
    ERROR_CODE_OFFSET + error as u32
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

pub fn rpc_transaction_error(error: &RpcError) -> Option<TransactionError> {
    match error {
        RpcError::TransactionError(transaction_error) => Some(transaction_error.clone()),
        RpcError::ClientError(client_error) => client_error.get_transaction_error(),
        _ => None,
    }
}

pub fn rpc_custom_error_code(error: &RpcError) -> Option<u32> {
    match rpc_transaction_error(error) {
        Some(TransactionError::InstructionError(_, InstructionError::Custom(error_code))) => {
            Some(error_code)
        }
        _ => None,
    }
}

pub fn rpc_is_already_processed(error: &RpcError) -> bool {
    matches!(
        rpc_transaction_error(error),
        Some(TransactionError::AlreadyProcessed)
    )
}

impl From<tokio::task::JoinError> for ForesterError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Other(err.into())
    }
}
