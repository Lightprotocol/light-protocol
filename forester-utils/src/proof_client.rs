use std::time::Duration;

use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_prover_client::gnark::{
    constants::PROVE_PATH,
    proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
};
use reqwest::Client;
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::error::ForesterUtilsError;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ProofResponse {
    Async {
        job_id: String,
        estimated_time: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct JobStatusResponse {
    pub status: String,
    pub message: Option<String>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

pub struct ProofClient {
    client: Client,
    server_address: String,
    polling_interval: Duration,
    max_wait_time: Duration,
}

impl ProofClient {
    pub fn local() -> Self {
        Self {
            client: Client::new(),
            server_address: "http://localhost:3001".to_string(),
            polling_interval: Duration::from_secs(1),
            max_wait_time: Duration::from_secs(120),
        }
    }

    #[allow(unused)]
    pub fn with_config(
        server_address: String,
        polling_interval: Duration,
        max_wait_time: Duration,
    ) -> Self {
        Self {
            client: Client::new(),
            server_address,
            polling_interval,
            max_wait_time,
        }
    }

    pub async fn generate_proof(
        &self,
        inputs_json: String,
        circuit_type: &str,
    ) -> Result<CompressedProof, ForesterUtilsError> {
        let start_time = std::time::Instant::now();

        info!("Generating proof for circuit type: {}", circuit_type,);

        let request = self
            .client
            .post(format!("{}{}", self.server_address, PROVE_PATH))
            .header("Content-Type", "application/json");

        let response = request
            .body(inputs_json.clone())
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to prover server: {:?}", e);
                ForesterUtilsError::Prover(format!("Failed to send request: {}", e))
            })?;

        let status_code = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read response body: {:?}", e);
            ForesterUtilsError::Prover(format!("Failed to read response body: {}", e))
        })?;

        match status_code {
            reqwest::StatusCode::OK => {
                debug!("Received synchronous proof response");
                self.parse_proof_from_json(&response_text)
            }
            reqwest::StatusCode::ACCEPTED => {
                debug!("Received asynchronous job response");
                let job_response: ProofResponse =
                    serde_json::from_str(&response_text).map_err(|e| {
                        error!("Failed to parse async response: {:?}", e);
                        ForesterUtilsError::Prover(format!("Failed to parse async response: {}", e))
                    })?;

                match job_response {
                    ProofResponse::Async {
                        job_id,
                        estimated_time,
                        ..
                    } => {
                        info!(
                            "Proof job queued with ID: {}, estimated time: {:?}",
                            job_id, estimated_time
                        );
                        self.poll_for_result(&job_id, start_time).await
                    }
                }
            }
            _ => {
                if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
                    error!(
                        "Prover server error: {} - {}",
                        error_response.code, error_response.message
                    );
                    Err(ForesterUtilsError::Prover(format!(
                        "Prover server error: {} - {}",
                        error_response.code, error_response.message
                    )))
                } else {
                    error!("Prover server error: {}", response_text);
                    Err(ForesterUtilsError::Prover(format!(
                        "Prover server error: {}",
                        response_text
                    )))
                }
            }
        }
    }

    async fn poll_for_result(
        &self,
        job_id: &str,
        start_time: std::time::Instant,
    ) -> Result<CompressedProof, ForesterUtilsError> {
        let status_url = format!("{}/prove/status?job_id={}", self.server_address, job_id);

        loop {
            if start_time.elapsed() > self.max_wait_time {
                return Err(ForesterUtilsError::Prover(format!(
                    "Proof generation timed out after {:?} for job {}",
                    self.max_wait_time, job_id
                )));
            }

            let response = self.client.get(&status_url).send().await.map_err(|e| {
                error!("Failed to check job status: {:?}", e);
                ForesterUtilsError::Prover(format!("Failed to check job status: {}", e))
            })?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ForesterUtilsError::Prover(format!(
                    "Status check failed: {}",
                    error_text
                )));
            }

            let status_response: JobStatusResponse = response.json().await.map_err(|e| {
                error!("Failed to parse status response: {:?}", e);
                ForesterUtilsError::Prover(format!("Failed to parse status response: {}", e))
            })?;

            match status_response.status.as_str() {
                "completed" => {
                    info!(
                        "Proof completed for job {} after {:?}",
                        job_id,
                        start_time.elapsed()
                    );

                    return if let Some(result) = status_response.result {
                        let proof_json = serde_json::to_string(&result).map_err(|e| {
                            ForesterUtilsError::Prover(format!("Failed to serialize result: {}", e))
                        })?;
                        self.parse_proof_from_json(&proof_json)
                    } else {
                        Err(ForesterUtilsError::Prover(
                            "Job completed but no result provided".into(),
                        ))
                    };
                }
                "failed" => {
                    error!("Proof generation failed for job {}", job_id);
                    return Err(ForesterUtilsError::Prover(format!(
                        "Proof generation failed: {}",
                        status_response.message.unwrap_or_default()
                    )));
                }
                "processing" | "queued" => {
                    debug!(
                        "Job {} status: {} - waiting {:?} before next check",
                        job_id, status_response.status, self.polling_interval
                    );
                    sleep(self.polling_interval).await;
                }
                _ => {
                    warn!(
                        "Unknown job status '{}' for job {}, continuing to poll",
                        status_response.status, job_id
                    );
                    sleep(self.polling_interval).await;
                }
            }
        }
    }

    fn parse_proof_from_json(&self, json_str: &str) -> Result<CompressedProof, ForesterUtilsError> {
        let proof_json = deserialize_gnark_proof_json(json_str).map_err(|e| {
            error!("Failed to deserialize proof JSON: {:?}", e);
            ForesterUtilsError::Prover(format!("Failed to deserialize proof: {}", e))
        })?;

        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);

        Ok(CompressedProof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        })
    }

    pub async fn generate_batch_address_append_proof(
        &self,
        inputs: light_prover_client::batch_address_append::BatchAddressAppendInputs,
    ) -> Result<(CompressedProof, [u8; 32]), ForesterUtilsError> {
        let new_root =
            light_hasher::bigint::bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
        let inputs_json =
            light_prover_client::gnark::batch_address_append_json_formatter::to_json(&inputs);

        let proof = self.generate_proof(inputs_json, "address-append").await?;

        Ok((proof, new_root))
    }

    pub async fn generate_batch_append_proof(
        &self,
        circuit_inputs: light_prover_client::batch_append_with_proofs::BatchAppendWithProofsCircuitInputs,
    ) -> Result<(CompressedProof, [u8; 32]), ForesterUtilsError> {
        let inputs_json = light_prover_client::gnark::batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson::from_inputs(&circuit_inputs).to_string();
        let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(
            &circuit_inputs.new_root.to_biguint().unwrap(),
        )
        .unwrap();

        let proof = self
            .generate_proof(inputs_json, "append-with-proofs")
            .await?;

        Ok((proof, new_root))
    }

    pub async fn generate_batch_update_proof(
        &self,
        inputs: light_prover_client::batch_update::BatchUpdateCircuitInputs,
    ) -> Result<(CompressedProof, [u8; 32]), ForesterUtilsError> {
        let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(
            &inputs.new_root.to_biguint().unwrap(),
        )
        .map_err(|_| ForesterUtilsError::Prover("Failed to convert new root to bytes".into()))?;

        let json_str =
            light_prover_client::gnark::batch_update_json_formatter::update_inputs_string(&inputs);

        let proof = self.generate_proof(json_str, "update").await?;

        Ok((proof, new_root))
    }
}
