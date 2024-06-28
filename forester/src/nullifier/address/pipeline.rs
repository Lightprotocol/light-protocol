use crate::config::ForesterConfig;
use crate::nullifier::address::AddressProcessor;
use crate::nullifier::{BackpressureControl, ForesterQueueAccount, PipelineContext};
use light_test_utils::indexer::{Indexer, NewAddressProofWithContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum AddressPipelineStage<T: Indexer, R: RpcConnection> {
    FetchAddressQueueData(PipelineContext<T, R>),
    ProcessAddressQueue(PipelineContext<T, R>, Vec<ForesterQueueAccount>),
    UpdateAddressMerkleTree(PipelineContext<T, R>, ForesterQueueAccount),
    UpdateIndexer(PipelineContext<T, R>, Box<NewAddressProofWithContext>),
    Complete,
}

pub async fn setup_address_pipeline<T: Indexer, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<ForesterConfig>,
) -> (mpsc::Sender<AddressPipelineStage<T, R>>, mpsc::Receiver<()>) {
    let (input_tx, input_rx) = mpsc::channel(100);
    let (output_tx, mut output_rx) = mpsc::channel(100);
    let (completion_tx, completion_rx) = mpsc::channel(1);
    let (close_output_tx, close_output_rx) = mpsc::channel(1);
    let shutdown = Arc::new(AtomicBool::new(false));

    let mut processor = AddressProcessor {
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
    };

    tokio::spawn(async move {
        let processor_handle = tokio::spawn(async move {
            processor.process().await;
        });

        // Feed initial data into the pipeline
        input_tx_clone
            .send(AddressPipelineStage::FetchAddressQueueData(context.clone()))
            .await
            .unwrap();

        let mut consecutive_empty_fetches = 0;
        info!("Starting to process output in addresses_setup_pipeline");
        while let Some(result) = output_rx.recv().await {
            match result {
                AddressPipelineStage::FetchAddressQueueData(_) => {
                    input_tx_clone
                        .send(AddressPipelineStage::FetchAddressQueueData(context.clone()))
                        .await
                        .unwrap();
                }
                AddressPipelineStage::ProcessAddressQueue(_, ref queue_data) => {
                    if queue_data.is_empty() {
                        consecutive_empty_fetches += 1;
                        if consecutive_empty_fetches >= 1 {
                            debug!("No more addresses to process. Signaling completion.");
                            break;
                        }
                    } else {
                        consecutive_empty_fetches = 0;
                    }
                    input_tx_clone.send(result).await.unwrap();
                }
                AddressPipelineStage::Complete => {
                    debug!("Processing complete, signaling completion.");
                    break;
                }

                stage => {
                    input_tx_clone.send(stage).await.unwrap();
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
