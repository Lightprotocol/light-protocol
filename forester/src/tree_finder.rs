use crate::tree_data_sync::fetch_trees;
use crate::Result;
use forester_utils::forester_epoch::TreeAccounts;
use light_client::rpc::RpcConnection;
use light_client::rpc_pool::SolanaRpcPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

pub struct TreeFinder<R: RpcConnection> {
    rpc_pool: Arc<SolanaRpcPool<R>>,
    known_trees: Vec<TreeAccounts>,
    new_tree_sender: broadcast::Sender<TreeAccounts>,
    check_interval: Duration,
}

impl<R: RpcConnection> TreeFinder<R> {
    pub fn new(
        rpc_pool: Arc<SolanaRpcPool<R>>,
        initial_trees: Vec<TreeAccounts>,
        new_tree_sender: broadcast::Sender<TreeAccounts>,
        check_interval: Duration,
    ) -> Self {
        Self {
            rpc_pool,
            known_trees: initial_trees,
            new_tree_sender,
            check_interval,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut interval = interval(self.check_interval);

        loop {
            interval.tick().await;
            debug!("Checking for new trees");

            match self.check_for_new_trees().await {
                Ok(new_trees) => {
                    for tree in new_trees {
                        if let Err(e) = self.new_tree_sender.send(tree) {
                            error!("Failed to send new tree: {:?}", e);
                        } else {
                            info!("New tree discovered and sent: {:?}", tree);
                            self.known_trees.push(tree);
                        }
                    }
                }
                Err(e) => {
                    error!("Error checking for new trees: {:?}", e);
                }
            }

            tokio::task::yield_now().await;
        }
    }

    async fn check_for_new_trees(&self) -> Result<Vec<TreeAccounts>> {
        let rpc = self.rpc_pool.get_connection().await?;
        let current_trees = fetch_trees(&*rpc).await?;

        let new_trees: Vec<TreeAccounts> = current_trees
            .into_iter()
            .filter(|tree| !self.known_trees.contains(tree))
            .collect();

        Ok(new_trees)
    }
}
