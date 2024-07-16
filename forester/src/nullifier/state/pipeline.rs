use crate::config::ForesterConfig;
use crate::nullifier::state::StateProcessor;
use crate::nullifier::{BackpressureControl, ForesterQueueAccountData, PipelineContext};
use crate::rollover::RolloverState;
use crate::tree_sync::TreeData;
use crate::RpcPool;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::debug;
use log::info;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum StatePipelineStage<T: Indexer<R>, R: RpcConnection> {
    FetchStateQueueData(PipelineContext<T, R>),
    FetchProofs(PipelineContext<T, R>, Vec<ForesterQueueAccountData>),
    NullifyStateBatch(PipelineContext<T, R>, Vec<ForesterQueueAccountData>),
    Complete,
}

impl<T: Indexer<R>, R: RpcConnection> Display for StatePipelineStage<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatePipelineStage::FetchStateQueueData(_) => write!(f, "FetchStateQueueData"),
            StatePipelineStage::FetchProofs(_, _) => write!(f, "FetchProofs"),
            StatePipelineStage::NullifyStateBatch(_, _) => write!(f, "NullifyStateBatch"),
            StatePipelineStage::Complete => write!(f, "Complete"),
        }
    }
}

pub async fn setup_state_pipeline<T: Indexer<R>, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc_pool: RpcPool<R>,
    config: Arc<ForesterConfig>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) -> (mpsc::Sender<StatePipelineStage<T, R>>, mpsc::Receiver<()>) {
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
        state_queue: Arc::new(Mutex::new(Vec::new())),
    };

    let input_tx_clone = input_tx.clone();

    let context = PipelineContext {
        indexer: indexer.clone(),
        rpc_pool,
        config: config.clone(),
        tree_data,
        successful_nullifications: Arc::new(Mutex::new(0)),
        rollover_state: rollover_state.clone(),
    };

    tokio::spawn(async move {
        let processor_handle = tokio::spawn(async move {
            processor.process().await;
        });

        // Feed initial data into the pipeline
        input_tx_clone
            .send(StatePipelineStage::FetchStateQueueData(context.clone()))
            .await
            .unwrap();

        info!("Starting to process output in state_setup_pipeline");
        while let Some(result) = output_rx.recv().await {
            match result {
                StatePipelineStage::FetchStateQueueData(_) => {
                    input_tx_clone
                        .send(StatePipelineStage::FetchStateQueueData(context.clone()))
                        .await
                        .unwrap();
                }
                StatePipelineStage::FetchProofs(_, queue_data) => {
                    if queue_data.is_empty() {
                        input_tx_clone
                            .send(StatePipelineStage::FetchStateQueueData(context.clone()))
                            .await
                            .unwrap();
                    } else {
                        input_tx_clone
                            .send(StatePipelineStage::FetchProofs(context.clone(), queue_data))
                            .await
                            .unwrap();
                    }
                }
                StatePipelineStage::Complete => {
                    debug!("Processing complete, signaling completion.");
                    break;
                }

                stage => {
                    input_tx_clone.send(stage).await.unwrap();
                }
            }
        }

        processor_handle.abort();
        drop(output_tx);

        shutdown.store(true, Ordering::Relaxed);
        let _ = close_output_tx.send(()).await;
        let _ = completion_tx.send(()).await;
        debug!("Pipeline process completed.");
    });

    (input_tx, completion_rx)
}
