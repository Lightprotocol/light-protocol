#[derive(Debug, Clone)]
pub struct AddressTreeStrategy;

#[derive(Debug)]
pub struct AddressQueueData {
    pub staging_tree: FastAddressStagingTree,
    pub address_queue: light_client::indexer::AddressQueueDataV2,
}

#[async_trait]
impl<R: Rpc> TreeStrategy<R> for AddressTreeStrategy {
    type StagingTree = AddressQueueData;

    fn name(&self) -> &'static str {
        "Address"
    }

    fn circuit_type(&self, _queue_data: &Self::StagingTree) -> CircuitType {
        CircuitType::AddressAppend
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64> {
        fetch_address_zkp_batch_size(context).await
    }

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        _queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size_usize);
        let fetch_len = total_needed as u64;

        let address_queue =
            match fetch_address_batches(context, None, fetch_len, zkp_batch_size).await? {
                Some(aq) if !aq.addresses.is_empty() => aq,
                Some(_) => {
                    debug!("Address queue is empty");
                    return Ok(None);
                }
                None => {
                    debug!("No address queue data available");
                    return Ok(None);
                }
            };

        if address_queue.subtrees.is_empty() {
            return Err(anyhow!("Address queue missing subtrees data"));
        }

        let available = address_queue.addresses.len();
        let num_batches = (available / zkp_batch_size_usize).min(max_batches);

        if num_batches == 0 {
            debug!(
                "Not enough addresses for a complete batch: have {}, need {}",
                available, zkp_batch_size_usize
            );
            return Ok(None);
        }

        if address_queue.leaves_hash_chains.len() < num_batches {
            return Err(anyhow!(
                "Insufficient leaves_hash_chains: have {}, need {}",
                address_queue.leaves_hash_chains.len(),
                num_batches
            ));
        }

        let initial_root = address_queue.initial_root;
        let start_index = address_queue.start_index;
        let nodes_len = address_queue.nodes.len();

        let staging_tree = if !address_queue.nodes.is_empty() {
            FastAddressStagingTree::from_nodes(
                &address_queue.nodes,
                &address_queue.node_hashes,
                initial_root,
                start_index as usize,
            )?
        } else {
            FastAddressStagingTree::from_subtrees(
                address_queue.subtrees.to_vec(),
                start_index as usize,
                initial_root,
            )?
        };

        Ok(Some(QueueData {
            staging_tree: AddressQueueData {
                staging_tree,
                address_queue,
            },
            initial_root,
            num_batches,
        }))
    }

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let address_queue = &queue_data.address_queue;
        let range = batch_range(zkp_batch_size, address_queue.addresses.len(), start);
        let addresses = address_queue.addresses[range.clone()].to_vec();
        let zkp_batch_size_actual = addresses.len();

        let low_element_values = address_queue.low_element_values[range.clone()].to_vec();
        let low_element_next_values = address_queue.low_element_next_values[range.clone()].to_vec();
        let low_element_indices: Vec<usize> = address_queue.low_element_indices[range.clone()]
            .iter()
            .map(|&i| i as usize)
            .collect();
        let low_element_next_indices: Vec<usize> = address_queue.low_element_next_indices
            [range.clone()]
        .iter()
        .map(|&i| i as usize)
        .collect();
        let low_element_proofs = address_queue.low_element_proofs[range].to_vec();

        let leaves_hashchain = get_leaves_hashchain(&address_queue.leaves_hash_chains, batch_idx)?;

        let batch_start = Instant::now();
        let result = queue_data
            .staging_tree
            .process_batch(
                addresses,
                low_element_values,
                low_element_next_values,
                low_element_indices,
                low_element_next_indices,
                low_element_proofs,
                leaves_hashchain,
                zkp_batch_size_actual,
            )
            .map_err(|e| anyhow!("Failed to process address batch: {}", e))?;
        let batch_duration = batch_start.elapsed();

        let new_root = result.new_root;

        Ok((ProofInput::AddressAppend(result.circuit_inputs), new_root))
    }
}
