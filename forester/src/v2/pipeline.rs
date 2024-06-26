use std::sync::{Arc};
use log::{info, warn};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use crate::errors::ForesterError;
use crate::nullifier::Config;
use crate::v2::backpressure::BackpressureControl;
use crate::v2::queue_data::{AccountData, QueueData};
use crate::v2::stream_processor::StreamProcessor;

pub enum PipelineStage<T: Indexer, R: RpcConnection> {
    FetchQueueData(PipelineContext<T, R>),
    FetchProofs(PipelineContext<T, R>, QueueData),
    ProcessAccount(PipelineContext<T, R>, AccountData),
    NullifyAccount(PipelineContext<T, R>, AccountData),
    UpdateIndexer(PipelineContext<T, R>, AccountData),
}

pub struct PipelineContext<T: Indexer, R: RpcConnection> {
    pub indexer: Arc<Mutex<T>>,
    pub rpc: Arc<Mutex<R>>,
    pub config: Arc<Config>,
}

impl<T: Indexer, R: RpcConnection> Clone for PipelineContext<T, R> {
    fn clone(&self) -> Self {
        PipelineContext {
            indexer: Arc::clone(&self.indexer),
            rpc: Arc::clone(&self.rpc),
            config: Arc::clone(&self.config),
        }
    }
}

pub async fn setup_pipeline<T: Indexer, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<Config>,
) -> Result<(), ForesterError> {
    let (input_tx, input_rx) = mpsc::channel(100);
    let (output_tx, mut output_rx) = mpsc::channel(100);
    let (command_tx, mut command_rx) = mpsc::channel(100);

    let mut processor = StreamProcessor {
        input: input_rx,
        output: output_tx,
        backpressure: BackpressureControl::new(config.concurrency_limit),
    };

    let processor_handle = tokio::spawn(async move {
        processor.process().await;
    });

    let context = PipelineContext {
        indexer: indexer.clone(),
        rpc: rpc.clone(),
        config: config.clone(),
    };

    // Function to fetch queue data and send to processor
    async fn fetch_and_send<T: Indexer, R: RpcConnection>(
        context: PipelineContext<T, R>,
        input_tx: &mpsc::Sender<PipelineStage<T, R>>,
        command_tx: &mpsc::Sender<Result<(), ForesterError>>,
    ) -> Result<(), ForesterError> {
        match StreamProcessor::<T, R>::fetch_queue_data(context.clone()).await {
            Ok(PipelineStage::FetchProofs(_, queue_data)) if !queue_data.accounts_to_nullify.is_empty() => {
                input_tx.send(PipelineStage::FetchProofs(context, queue_data)).await.unwrap();
                Ok(())
            },
            Ok(_) => {
                info!("No accounts to nullify. Exiting.");
                command_tx.send(Ok(())).await.unwrap();
                Err(ForesterError::Custom("No accounts to nullify".to_string()))
            },
            Err(e) => {
                warn!("Error fetching queue data: {:?}", e);
                let owned_error = e.to_owned();
                command_tx.send(Err(owned_error)).await.unwrap();
                Err(e)
            }
        }
    }

    // Initial fetch
    fetch_and_send(context.clone(), &input_tx, &command_tx).await?;

    loop {
        tokio::select! {
            Some(result) = output_rx.recv() => {
                if let PipelineStage::FetchQueueData(_) = result {
                    // We've completed a full cycle, check if there are more accounts to nullify
                    match fetch_and_send(context.clone(), &input_tx, &command_tx).await {
                        Ok(_) => {},
                        Err(ref e) => {
                            if let ForesterError::Custom(ref msg) = e {
                                if msg == "No accounts to nullify" {
                                    break;
                                }
                            }
                             let owned_error = e.to_owned();
                            return Err(owned_error);
                        }
                    }
                }
            }
            Some(command) = command_rx.recv() => {
                match command {
                    Ok(_) => break,
                    Err(e) => return Err(e),
                }
            }
            else => break,
        }
    }

    // Ensure the processor task is properly shut down
    processor_handle.abort();

    Ok(())
}
