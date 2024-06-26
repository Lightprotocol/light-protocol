use std::sync::Arc;
use log::info;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use crate::nullifier::Config;
use crate::v2::state::backpressure::BackpressureControl;
use crate::v2::state::queue_data::{AccountData, QueueData};
use crate::v2::state::stream_processor::StreamProcessor;

#[derive(Debug)]
pub enum PipelineStage<T: Indexer, R: RpcConnection> {
    FetchQueueData(PipelineContext<T, R>),
    FetchProofs(PipelineContext<T, R>, QueueData),
    NullifyAccount(PipelineContext<T, R>, AccountData),
    UpdateIndexer(PipelineContext<T, R>, AccountData),
}

#[derive(Debug)]
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
) -> (mpsc::Sender<PipelineStage<T, R>>, mpsc::Receiver<()>) {
    let (input_tx, input_rx) = mpsc::channel(100);
    let (output_tx, mut output_rx) = mpsc::channel(100);
    let (completion_tx, completion_rx) = mpsc::channel(1);

    let mut processor = StreamProcessor {
        input: input_rx,
        output: output_tx,
        backpressure: BackpressureControl::new(config.concurrency_limit),
    };

    let input_tx_clone = input_tx.clone();
    let context = PipelineContext {
        indexer: indexer.clone(),
        rpc: rpc.clone(),
        config: config.clone(),
    };

    tokio::spawn(async move {
        let processor_handle = tokio::spawn(async move {
            processor.process().await;
        });

        // Feed initial data into the pipeline
        input_tx_clone.send(PipelineStage::FetchQueueData(context.clone())).await.unwrap();

        info!("Starting to process output in setup_pipeline");
        while let Some(result) = output_rx.recv().await {
            info!("Received result in setup_pipeline: {:?}", result);
            match result {
                PipelineStage::FetchQueueData(_) => {
                    info!("Received FetchQueueData, restarting pipeline");
                    input_tx_clone.send(PipelineStage::FetchQueueData(context.clone())).await.unwrap();
                }
                PipelineStage::FetchProofs(_, queue_data) => {
                    if queue_data.accounts_to_nullify.is_empty() {
                        info!("No more accounts to nullify after 3 consecutive empty fetches. Signaling completion.");
                        break;
                    } else {
                        info!("Received FetchProofs in setup_pipeline, processing {} accounts", queue_data.accounts_to_nullify.len());
                        input_tx_clone.send(PipelineStage::FetchProofs(context.clone(), queue_data)).await.unwrap();
                    }
                }
                PipelineStage::NullifyAccount(_, account_data) => {
                    info!("Received NullifyAccount for account: {} in setup_pipeline", account_data.account.hash_string());
                    input_tx_clone.send(PipelineStage::NullifyAccount(context.clone(), account_data)).await.unwrap();
                }
                PipelineStage::UpdateIndexer(_, account_data) => {
                    info!("Received UpdateIndexer for account: {} in setup_pipeline", account_data.account.hash_string());
                    input_tx_clone.send(PipelineStage::UpdateIndexer(context.clone(), account_data)).await.unwrap();
                }
            }
        }
        // Ensure the processor task is properly shut down
        processor_handle.abort();

        let _ = completion_tx.send(()).await;
        info!("Pipeline process completed.");
    });

    (input_tx, completion_rx)
}
