use std::sync::Arc;

/// Proof generation logic for batched operations.
use anyhow::Result;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
    },
};
use tracing::{debug, info};

/// Configuration for proof generation.
#[derive(Clone)]
pub struct ProofConfig {
    pub append_url: String,
    pub update_url: String,
    pub polling_interval: std::time::Duration,
    pub max_wait_time: std::time::Duration,
    pub api_key: Option<String>,
}

/// Generates append proofs in parallel for multiple batches.
pub async fn generate_append_proofs(
    circuit_inputs: Vec<BatchAppendsCircuitInputs>,
    config: &ProofConfig,
) -> Result<Vec<InstructionDataBatchAppendInputs>> {
    if circuit_inputs.is_empty() {
        return Ok(vec![]);
    }

    let proof_client = Arc::new(ProofClient::with_config(
        config.append_url.clone(),
        config.polling_interval,
        config.max_wait_time,
        config.api_key.clone(),
    ));

    let num_proofs = circuit_inputs.len();
    info!("Generating {} append proofs in parallel", num_proofs);

    let proof_futures: Vec<_> = circuit_inputs
        .into_iter()
        .enumerate()
        .map(|(idx, circuit_input)| {
            let client = proof_client.clone();
            tokio::spawn(async move {
                debug!("Starting append proof generation for batch {}", idx);
                let result = client.generate_batch_append_proof(circuit_input).await;
                debug!("Completed append proof generation for batch {}", idx);
                result
            })
        })
        .collect();

    let results = futures::future::join_all(proof_futures).await;

    let mut proofs = Vec::with_capacity(num_proofs);
    for (idx, result) in results.into_iter().enumerate() {
        let (proof, new_root) = result
            .map_err(|e| anyhow::anyhow!("Task join error for batch {}: {}", idx, e))?
            .map_err(|e| anyhow::anyhow!("Prover error for batch {}: {}", idx, e))?;

        proofs.push(InstructionDataBatchAppendInputs {
            new_root,
            compressed_proof:
                light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
        });
    }

    info!("Generated {} append proofs successfully", proofs.len());
    Ok(proofs)
}

/// Generates nullify proofs in parallel for multiple batches.
pub async fn generate_nullify_proofs(
    circuit_inputs: Vec<BatchUpdateCircuitInputs>,
    config: &ProofConfig,
) -> Result<Vec<InstructionDataBatchNullifyInputs>> {
    if circuit_inputs.is_empty() {
        return Ok(vec![]);
    }

    let proof_client = Arc::new(ProofClient::with_config(
        config.update_url.clone(),
        config.polling_interval,
        config.max_wait_time,
        config.api_key.clone(),
    ));

    let num_proofs = circuit_inputs.len();
    info!("Generating {} nullify proofs in parallel", num_proofs);

    let proof_futures: Vec<_> = circuit_inputs
        .into_iter()
        .enumerate()
        .map(|(idx, circuit_input)| {
            let client = proof_client.clone();
            tokio::spawn(async move {
                debug!("Starting nullify proof generation for batch {}", idx);
                let result = client.generate_batch_update_proof(circuit_input).await;
                debug!("Completed nullify proof generation for batch {}", idx);
                result
            })
        })
        .collect();

    let results = futures::future::join_all(proof_futures).await;

    let mut proofs = Vec::with_capacity(num_proofs);
    for (idx, result) in results.into_iter().enumerate() {
        let (proof, new_root) = result
            .map_err(|e| anyhow::anyhow!("Task join error for batch {}: {}", idx, e))?
            .map_err(|e| anyhow::anyhow!("Prover error for batch {}: {}", idx, e))?;

        proofs.push(InstructionDataBatchNullifyInputs {
            new_root,
            compressed_proof:
                light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
        });
    }

    info!("Generated {} nullify proofs successfully", proofs.len());
    Ok(proofs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_proof_generation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let config = ProofConfig {
            append_url: "http://localhost:8080".to_string(),
            update_url: "http://localhost:8081".to_string(),
            polling_interval: std::time::Duration::from_millis(100),
            max_wait_time: std::time::Duration::from_secs(30),
            api_key: None,
        };

        let result = rt.block_on(generate_append_proofs(vec![], &config));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        let result = rt.block_on(generate_nullify_proofs(vec![], &config));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
