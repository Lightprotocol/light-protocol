use crate::config::ForesterConfig;
use crate::v2::address::AddressProcessor;
use crate::v2::BackpressureControl;
use light_test_utils::indexer::{Indexer, NewAddressProofWithContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum AddressPipelineStage<T: Indexer, R: RpcConnection> {
    FetchAddressQueueData(PipelineContext<T, R>),
    ProcessAddressQueue(PipelineContext<T, R>, Vec<crate::v2::address::Account>),
    UpdateAddressMerkleTree(PipelineContext<T, R>, crate::v2::address::Account),
    UpdateIndexer(PipelineContext<T, R>, Box<NewAddressProofWithContext>),
}

#[derive(Debug)]
pub struct PipelineContext<T: Indexer, R: RpcConnection> {
    pub indexer: Arc<Mutex<T>>,
    pub rpc: Arc<Mutex<R>>,
    pub config: Arc<ForesterConfig>,
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

pub async fn setup_address_pipeline<T: Indexer, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<ForesterConfig>,
) -> (mpsc::Sender<AddressPipelineStage<T, R>>, mpsc::Receiver<()>) {
    let (input_tx, input_rx) = mpsc::channel(100);
    let (output_tx, mut output_rx) = mpsc::channel(100);
    let (completion_tx, completion_rx) = mpsc::channel(1);

    let mut processor = AddressProcessor {
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
        input_tx_clone
            .send(AddressPipelineStage::FetchAddressQueueData(context.clone()))
            .await
            .unwrap();

        let mut consecutive_empty_fetches = 0;
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
                            info!("No more addresses to process after 3 consecutive empty fetches. Signaling completion.");
                            break;
                        }
                    } else {
                        consecutive_empty_fetches = 0;
                    }
                    input_tx_clone.send(result).await.unwrap();
                }
                stage => {
                    input_tx_clone.send(stage).await.unwrap();
                }
            }
        }

        processor_handle.abort();
        let _ = completion_tx.send(()).await;
        info!("Address pipeline process completed.");
    });

    (input_tx, completion_rx)
}
