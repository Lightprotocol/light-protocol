use crate::config::ForesterConfig;
use crate::nullifier::state::StateProcessor;
use crate::nullifier::{
    BackpressureControl, ForesterQueueAccountData, ForesterQueueData, PipelineContext,
};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum PipelineStage<T: Indexer<R>, R: RpcConnection> {
    FetchQueueData(PipelineContext<T, R>),
    FetchProofs(PipelineContext<T, R>, ForesterQueueData),
    NullifyAccount(PipelineContext<T, R>, ForesterQueueAccountData),
    UpdateIndexer(PipelineContext<T, R>, ForesterQueueAccountData),
    Complete,
}

pub async fn setup_state_pipeline<T: Indexer<R>, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<ForesterConfig>,
) -> (mpsc::Sender<PipelineStage<T, R>>, mpsc::Receiver<()>) {
    let (input_tx, input_rx) = mpsc::channel(100);
    let (output_tx, mut output_rx) = mpsc::channel(100);
    let (completion_tx, completion_rx) = mpsc::channel(1);
    let (close_output_tx, close_output_rx) = mpsc::channel(1);
    let shutdown = Arc::new(AtomicBool::new(false));

    let mut processor = StateProcessor {
        input: input_rx,
        output: output_tx.clone(),
        backpressure: BackpressureControl::new(config.concurrency_limit),
        shutdown: shutdown.clone(),
        close_output: close_output_rx,
    };

    let input_tx_clone = input_tx.clone();
    let context = PipelineContext {
        indexer: indexer.clone(),
        rpc: rpc.clone(),
        config: config.clone(),
        successful_nullifications: Arc::new(Mutex::new(0)),
    };

    tokio::spawn(async move {
        let processor_handle = tokio::spawn(async move {
            processor.process().await;
        });

        // Feed initial data into the pipeline
        input_tx_clone
            .send(PipelineStage::FetchQueueData(context.clone()))
            .await
            .unwrap();

        debug!("Starting to process output in state_setup_pipeline");
        while let Some(result) = output_rx.recv().await {
            debug!("Received result in state_setup_pipeline: {:?}", result);
            match result {
                PipelineStage::FetchQueueData(_) => {
                    debug!("Received FetchQueueData, restarting pipeline");
                    input_tx_clone
                        .send(PipelineStage::FetchQueueData(context.clone()))
                        .await
                        .unwrap();
                }
                PipelineStage::FetchProofs(_, queue_data) => {
                    if queue_data.accounts_to_nullify.is_empty() {
                        debug!("No more accounts to nullify. Signaling completion.");
                        input_tx_clone.send(PipelineStage::Complete).await.unwrap();
                    } else {
                        debug!(
                            "Received FetchProofs in setup_pipeline, processing {} accounts",
                            queue_data.accounts_to_nullify.len()
                        );
                        input_tx_clone
                            .send(PipelineStage::FetchProofs(context.clone(), queue_data))
                            .await
                            .unwrap();
                    }
                }
                PipelineStage::NullifyAccount(_, account_data) => {
                    debug!(
                        "Received NullifyAccount for account: {} in setup_pipeline",
                        account_data.account.hash_string()
                    );
                    input_tx_clone
                        .send(PipelineStage::NullifyAccount(context.clone(), account_data))
                        .await
                        .unwrap();
                }
                PipelineStage::UpdateIndexer(_, account_data) => {
                    debug!(
                        "Received UpdateIndexer for account: {} in setup_pipeline",
                        account_data.account.hash_string()
                    );
                    input_tx_clone
                        .send(PipelineStage::UpdateIndexer(context.clone(), account_data))
                        .await
                        .unwrap();
                }
                PipelineStage::Complete => {
                    debug!("Processing complete, signaling completion.");
                    shutdown.store(true, Ordering::Relaxed);
                    let _ = close_output_tx.send(()).await;
                    break;
                }
            }
        }
        // Ensure the processor task is properly shut down
        processor_handle.abort();

        // Close the output channel
        drop(output_tx);

        shutdown.store(true, Ordering::Relaxed);
        let _ = close_output_tx.send(()).await;
        let _ = completion_tx.send(()).await;
        debug!("Pipeline process completed.");
    });

    (input_tx, completion_rx)
}
