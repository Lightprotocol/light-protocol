use std::time::{Duration, Instant};

use reqwest::Client;
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::{
    constants::PROVE_PATH,
    errors::ProverClientError,
    proof::{
        compress_proof, deserialize_gnark_proof_json, proof_from_json_struct, ProofCompressed,
    },
    proof_types::{
        batch_address_append::{to_json, BatchAddressAppendInputs},
        batch_append::{BatchAppendInputsJson, BatchAppendsCircuitInputs},
        batch_update::{update_inputs_string, BatchUpdateCircuitInputs},
    },
};

const MAX_RETRIES: u32 = 10;
const BASE_RETRY_DELAY_SECS: u64 = 1;
const DEFAULT_POLLING_INTERVAL_SECS: u64 = 1;
const DEFAULT_MAX_WAIT_TIME_SECS: u64 = 600;
const DEFAULT_LOCAL_SERVER: &str = "http://localhost:3001";

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
    api_key: Option<String>,
}

impl ProofClient {
    pub fn local() -> Self {
        Self {
            client: Client::new(),
            server_address: DEFAULT_LOCAL_SERVER.to_string(),
            polling_interval: Duration::from_secs(DEFAULT_POLLING_INTERVAL_SECS),
            max_wait_time: Duration::from_secs(DEFAULT_MAX_WAIT_TIME_SECS),
            api_key: None,
        }
    }

    #[allow(unused)]
    pub fn with_config(
        server_address: String,
        polling_interval: Duration,
        max_wait_time: Duration,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            server_address,
            polling_interval,
            max_wait_time,
            api_key,
        }
    }

    pub async fn generate_proof(
        &self,
        inputs_json: String,
        circuit_type: &str,
    ) -> Result<ProofCompressed, ProverClientError> {
        let start_time = Instant::now();
        let mut retries = 0;

        loop {
            let elapsed = start_time.elapsed();
            if elapsed > self.max_wait_time {
                return Err(ProverClientError::ProverServerError(format!(
                    "Overall proof generation timed out after {:?} (max: {:?}), retries: {}",
                    elapsed, self.max_wait_time, retries
                )));
            }

            match self
                .try_generate_proof(&inputs_json, circuit_type, retries + 1, elapsed)
                .await
            {
                Ok(proof) => return Ok(proof),
                Err(err) if self.should_retry(&err, retries, elapsed) => {
                    retries += 1;
                    let retry_delay = Duration::from_secs(BASE_RETRY_DELAY_SECS * retries as u64);

                    if elapsed + retry_delay > self.max_wait_time {
                        warn!(
                            "Skipping retry due to max wait time constraint: elapsed={:?}, retry_delay={:?}, max_wait={:?}",
                            elapsed, retry_delay, self.max_wait_time
                        );
                        return Err(err);
                    }

                    warn!(
                        "Retrying proof generation ({}/{}) after {:?} due to: {}",
                        retries, MAX_RETRIES, retry_delay, err
                    );
                    sleep(retry_delay).await;
                }
                Err(err) => {
                    debug!(
                        "Not retrying error (retries={}, elapsed={:?}): {}",
                        retries, elapsed, err
                    );
                    return Err(err);
                }
            }
        }
    }

    async fn try_generate_proof(
        &self,
        inputs_json: &str,
        circuit_type: &str,
        attempt: u32,
        elapsed: Duration,
    ) -> Result<ProofCompressed, ProverClientError> {
        debug!(
            "Generating proof for circuit type: {} (attempt {}, elapsed: {:?})",
            circuit_type, attempt, elapsed
        );

        let response = self.send_proof_request(inputs_json).await?;
        let status_code = response.status();
        let response_text = response.text().await.map_err(|e| {
            ProverClientError::ProverServerError(format!("Failed to read response body: {}", e))
        })?;

        self.log_response(status_code, &response_text);
        self.handle_proof_response(status_code, &response_text, elapsed)
            .await
    }

    async fn send_proof_request(
        &self,
        inputs_json: &str,
    ) -> Result<reqwest::Response, ProverClientError> {
        let url = format!("{}{}", self.server_address, PROVE_PATH);

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("X-API-Key", api_key);
        }

        request
            .body(inputs_json.to_string())
            .send()
            .await
            .map_err(|e| {
                ProverClientError::ProverServerError(format!(
                    "Failed to send request to prover server: {}",
                    e
                ))
            })
    }

    fn log_response(&self, status_code: reqwest::StatusCode, response_text: &str) {
        debug!("Response status: {}", status_code);
        debug!("Response text: {}", response_text);

        if !status_code.is_success() {
            error!("HTTP error: status={}, body={}", status_code, response_text);
        }
    }

    async fn handle_proof_response(
        &self,
        status_code: reqwest::StatusCode,
        response_text: &str,
        start_elapsed: Duration,
    ) -> Result<ProofCompressed, ProverClientError> {
        match status_code {
            reqwest::StatusCode::OK => {
                debug!("Received synchronous proof response");
                self.parse_proof_from_json(response_text)
            }
            reqwest::StatusCode::ACCEPTED => {
                debug!("Received asynchronous job response");
                let job_response = self.parse_job_response(response_text)?;
                self.handle_async_job(job_response, start_elapsed).await
            }
            _ => self.handle_error_response(response_text),
        }
    }

    fn parse_job_response(&self, response_text: &str) -> Result<ProofResponse, ProverClientError> {
        serde_json::from_str(response_text).map_err(|e| {
            error!("Failed to parse async response: {}", e);
            ProverClientError::ProverServerError(format!("Failed to parse async response: {}", e))
        })
    }

    async fn handle_async_job(
        &self,
        job_response: ProofResponse,
        start_elapsed: Duration,
    ) -> Result<ProofCompressed, ProverClientError> {
        match job_response {
            ProofResponse::Async { job_id, .. } => {
                info!("Proof job queued with ID: {}", job_id);
                self.poll_for_result(&job_id, start_elapsed).await
            }
        }
    }

    fn handle_error_response(
        &self,
        response_text: &str,
    ) -> Result<ProofCompressed, ProverClientError> {
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(response_text) {
            error!(
                "Prover server error: {} - {}",
                error_response.code, error_response.message
            );
            Err(ProverClientError::ProverServerError(format!(
                "Prover server error: {} - {}",
                error_response.code, error_response.message
            )))
        } else {
            error!("Prover server error: {}", response_text);
            Err(ProverClientError::ProverServerError(format!(
                "Prover server error: {}",
                response_text
            )))
        }
    }

    fn should_retry(&self, error: &ProverClientError, retries: u32, elapsed: Duration) -> bool {
        let error_str = error.to_string();
        let is_retryable_error = error_str.contains("job_not_found")
            || error_str.contains("connection")
            || error_str.contains("timeout")
            || error_str.contains("503")
            || error_str.contains("502")
            || error_str.contains("500");
        let should_retry =
            retries < MAX_RETRIES && is_retryable_error && elapsed < self.max_wait_time;

        debug!(
            "Retry check: retries={}/{}, is_retryable_error={}, elapsed={:?}/{:?}, should_retry={}, error={}",
            retries, MAX_RETRIES, is_retryable_error, elapsed, self.max_wait_time, should_retry, error_str
        );

        should_retry
    }

    async fn poll_for_result(
        &self,
        job_id: &str,
        start_elapsed: Duration,
    ) -> Result<ProofCompressed, ProverClientError> {
        let poll_start_time = Instant::now();
        let status_url = format!("{}/prove/status?job_id={}", self.server_address, job_id);

        info!("Starting to poll for job {} at URL: {}", job_id, status_url);

        let mut poll_count = 0;
        let mut transient_error_count = 0;

        loop {
            poll_count += 1;
            let poll_elapsed = poll_start_time.elapsed();
            let total_elapsed = start_elapsed + poll_elapsed;

            if total_elapsed > self.max_wait_time {
                return Err(ProverClientError::ProverServerError(format!(
                    "Job {} timed out after {:?} total (max: {:?}), polling time: {:?}, total polls: {}",
                    job_id, total_elapsed, self.max_wait_time, poll_elapsed, poll_count
                )));
            }

            debug!(
                "Poll #{} for job {} at total elapsed time {:?} (polling: {:?})",
                poll_count, job_id, total_elapsed, poll_elapsed
            );

            match self.poll_job_status(&status_url, job_id, poll_count).await {
                Ok(response) => {
                    transient_error_count = 0;

                    if let Some(proof) = self
                        .handle_job_status(response, job_id, total_elapsed, poll_count)
                        .await?
                    {
                        return Ok(proof);
                    }

                    if total_elapsed + self.polling_interval > self.max_wait_time {
                        warn!(
                            "Skipping polling interval due to max wait time constraint: total_elapsed={:?}, polling_interval={:?}, max_wait={:?}",
                            total_elapsed, self.polling_interval, self.max_wait_time
                        );
                        return Err(ProverClientError::ProverServerError(format!(
                            "Job {} polling stopped due to max wait time constraint",
                            job_id
                        )));
                    }

                    sleep(self.polling_interval).await;
                }
                Err(err) if self.is_job_not_found_error(&err) => {
                    error!(
                        "Job {} not found during polling - will retry with new proof request at higher level: {}",
                        job_id, err
                    );
                    return Err(err);
                }
                Err(err) if self.is_transient_polling_error(&err) => {
                    transient_error_count += 1;

                    debug!(
                        "Transient polling error for job {}: attempt {}/{}, error: {}",
                        job_id, transient_error_count, MAX_RETRIES, err
                    );

                    if transient_error_count >= MAX_RETRIES {
                        error!(
                            "Job {} polling failed after {} transient errors, giving up",
                            job_id, transient_error_count
                        );
                        return Err(err);
                    }

                    let retry_delay =
                        Duration::from_secs(BASE_RETRY_DELAY_SECS * transient_error_count as u64);

                    if total_elapsed + retry_delay > self.max_wait_time {
                        warn!(
                            "Skipping transient error retry due to max wait time constraint: total_elapsed={:?}, retry_delay={:?}, max_wait={:?}",
                            total_elapsed, retry_delay, self.max_wait_time
                        );
                        return Err(err);
                    }

                    warn!(
                        "Job {} transient error (attempt {}/{}), retrying after {:?}",
                        job_id, transient_error_count, MAX_RETRIES, retry_delay
                    );
                    sleep(retry_delay).await;
                }
                Err(err) => {
                    debug!("Not retrying polling error for job {}: {}", job_id, err);
                    return Err(err);
                }
            }
        }
    }

    async fn poll_job_status(
        &self,
        status_url: &str,
        job_id: &str,
        poll_count: u32,
    ) -> Result<JobStatusResponse, ProverClientError> {
        let mut request = self.client.get(status_url);

        if let Some(api_key) = &self.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await.map_err(|e| {
            error!("Failed to send status request for job {}: {}", job_id, e);
            ProverClientError::ProverServerError(format!("Failed to check job status: {}", e))
        })?;

        let status_code = response.status();
        let response_text = response.text().await.unwrap_or_default();

        debug!(
            "Poll #{} for job {}: status={}, body_len={}",
            poll_count,
            job_id,
            status_code,
            response_text.len()
        );

        if !status_code.is_success() {
            return Err(ProverClientError::ProverServerError(format!(
                "HTTP error while polling for result: status={}, body={}",
                status_code, response_text
            )));
        }

        serde_json::from_str(&response_text).map_err(|e| {
            error!(
                "Failed to parse status response on poll #{} for job {}: error={}, body={}",
                poll_count, job_id, e, response_text
            );
            ProverClientError::ProverServerError(format!(
                "Failed to parse status response: {}, body: {}",
                e, response_text
            ))
        })
    }

    async fn handle_job_status(
        &self,
        status_response: JobStatusResponse,
        job_id: &str,
        elapsed: Duration,
        poll_count: u32,
    ) -> Result<Option<ProofCompressed>, ProverClientError> {
        info!(
            "Poll #{} for job {}: status='{}', message='{}'",
            poll_count,
            job_id,
            status_response.status,
            status_response.message.as_deref().unwrap_or("none")
        );

        match status_response.status.as_str() {
            "completed" => {
                info!(
                    "Job {} completed successfully after {:?} and {} polls",
                    job_id, elapsed, poll_count
                );
                self.extract_proof_from_result(status_response.result, job_id)
                    .map(Some)
            }
            "failed" => {
                let error_msg = status_response
                    .message
                    .unwrap_or_else(|| "No error message provided".to_string());
                error!(
                    "Job {} failed after {:?} and {} polls: {}",
                    job_id, elapsed, poll_count, error_msg
                );
                Err(ProverClientError::ProverServerError(format!(
                    "Proof job {} failed: {}",
                    job_id, error_msg
                )))
            }
            "processing" | "queued" => {
                debug!(
                    "Job {} still {} after {:?} (poll #{}), waiting {:?} before next check",
                    job_id, status_response.status, elapsed, poll_count, self.polling_interval
                );
                Ok(None)
            }
            _ => {
                warn!(
                    "Job {} has unknown status '{}' on poll #{} after {:?}, continuing to poll",
                    job_id, status_response.status, poll_count, elapsed
                );
                Ok(None)
            }
        }
    }

    fn extract_proof_from_result(
        &self,
        result: Option<serde_json::Value>,
        job_id: &str,
    ) -> Result<ProofCompressed, ProverClientError> {
        match result {
            Some(result) => {
                debug!("Job {} has result, parsing proof JSON", job_id);
                let proof_json = serde_json::to_string(&result).map_err(|e| {
                    error!("Failed to serialize result for job {}: {}", job_id, e);
                    ProverClientError::ProverServerError("Cannot serialize result".to_string())
                })?;
                self.parse_proof_from_json(&proof_json)
            }
            None => {
                error!("Job {} completed but has no result", job_id);
                Err(ProverClientError::ProverServerError(
                    "No result in completed job status".to_string(),
                ))
            }
        }
    }

    fn is_job_not_found_error(&self, error: &ProverClientError) -> bool {
        error.to_string().contains("job_not_found")
    }

    fn is_transient_polling_error(&self, error: &ProverClientError) -> bool {
        let error_str = error.to_string();
        error_str.contains("503") || error_str.contains("502") || error_str.contains("500")
    }

    fn parse_proof_from_json(&self, json_str: &str) -> Result<ProofCompressed, ProverClientError> {
        let proof_json = deserialize_gnark_proof_json(json_str).map_err(|e| {
            ProverClientError::ProverServerError(format!("Failed to deserialize proof JSON: {}", e))
        })?;

        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);

        Ok(ProofCompressed {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        })
    }

    pub async fn generate_batch_address_append_proof(
        &self,
        inputs: BatchAddressAppendInputs,
    ) -> Result<(ProofCompressed, [u8; 32]), ProverClientError> {
        let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(&inputs.new_root)?;
        let inputs_json = to_json(&inputs);
        let proof = self.generate_proof(inputs_json, "address-append").await?;
        Ok((proof, new_root))
    }

    pub async fn generate_batch_append_proof(
        &self,
        circuit_inputs: BatchAppendsCircuitInputs,
    ) -> Result<(ProofCompressed, [u8; 32]), ProverClientError> {
        let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(
            &circuit_inputs.new_root.to_biguint().unwrap(),
        )?;
        let inputs_json = BatchAppendInputsJson::from_inputs(&circuit_inputs).to_string();
        let proof = self.generate_proof(inputs_json, "append").await?;
        Ok((proof, new_root))
    }

    pub async fn generate_batch_update_proof(
        &self,
        inputs: BatchUpdateCircuitInputs,
    ) -> Result<(ProofCompressed, [u8; 32]), ProverClientError> {
        let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(
            &inputs.new_root.to_biguint().unwrap(),
        )?;
        let json_str = update_inputs_string(&inputs);
        let proof = self.generate_proof(json_str, "update").await?;
        Ok((proof, new_root))
    }
}
