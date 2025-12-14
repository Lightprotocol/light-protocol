package server

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	v1 "light/light-prover/prover/v1"
	v2 "light/light-prover/prover/v2"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/redis/go-redis/v9"

	"github.com/gorilla/handlers"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

type proofStatusHandler struct {
	redisQueue *RedisQueue
}

func (handler proofStatusHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}

	jobID := r.URL.Query().Get("job_id")
	if jobID == "" {
		malformedBodyError(fmt.Errorf("job_id parameter required")).send(w)
		return
	}

	if !isValidJobID(jobID) {
		notFoundError := &Error{
			StatusCode: http.StatusBadRequest,
			Code:       "invalid_job_id",
			Message:    "Invalid job ID format. Job ID must be a valid UUID.",
		}
		notFoundError.send(w)
		return
	}

	logging.Logger().Info().
		Str("job_id", jobID).
		Msg("Checking job status")

	result, err := handler.redisQueue.GetResult(jobID)
	if err != nil && err != redis.Nil {
		logging.Logger().Error().
			Err(err).
			Str("job_id", jobID).
			Msg("Error retrieving result")
		unexpectedError(err).send(w)
		return
	}

	if err == nil && result != nil {
		logging.Logger().Info().
			Str("job_id", jobID).
			Msg("Job completed - returning result")

		response := map[string]interface{}{
			"job_id": jobID,
			"status": "completed",
			"result": result,
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		err := json.NewEncoder(w).Encode(response)
		if err != nil {
			return
		}
		return
	}

	jobExists, jobStatus, jobInfo := handler.checkJobExistsDetailed(jobID)

	if !jobExists {
		// Fallback: check job metadata - this catches jobs that were submitted but not yet
		// visible in queues due to Redis replica lag or race conditions
		jobMeta, metaErr := handler.redisQueue.GetJobMeta(jobID)
		if metaErr != nil {
			logging.Logger().Warn().
				Err(metaErr).
				Str("job_id", jobID).
				Msg("Error checking job metadata")
		}

		if jobMeta != nil {
			// Job was submitted (we have metadata) but not found in queues - return queued status
			logging.Logger().Info().
				Str("job_id", jobID).
				Interface("job_meta", jobMeta).
				Msg("Job not found in queues but metadata exists - returning queued status")

			response := map[string]interface{}{
				"job_id":  jobID,
				"status":  "queued",
				"message": "Job is queued and waiting to be processed",
			}
			if circuitType, ok := jobMeta["circuit_type"]; ok {
				response["circuit_type"] = circuitType
			}
			if submittedAt, ok := jobMeta["submitted_at"]; ok {
				response["submitted_at"] = submittedAt
			}

			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusAccepted)
			json.NewEncoder(w).Encode(response)
			return
		}

		logging.Logger().Warn().
			Str("job_id", jobID).
			Msg("Job not found in any queue or metadata")

		if handler.redisQueue != nil && handler.redisQueue.Client != nil {
			if stats, statsErr := handler.redisQueue.GetQueueStats(); statsErr == nil {
				logging.Logger().Info().
					Str("job_id", jobID).
					Interface("queue_stats", stats).
					Str("redis_addr", handler.redisQueue.Client.Options().Addr).
					Msg("Queue stats at job_not_found")
			} else {
				logging.Logger().Warn().
					Err(statsErr).
					Str("job_id", jobID).
					Msg("Failed to fetch queue stats during job_not_found")
			}
		}

		// Clean up any stale in-flight marker for this job ID
		// This allows new requests with the same input to create fresh jobs
		handler.redisQueue.CleanupStaleInFlightMarker(jobID)

		notFoundError := &Error{
			StatusCode: http.StatusNotFound,
			Code:       "job_not_found",
			Message:    fmt.Sprintf("Job with ID %s not found. It may have expired or never existed.", jobID),
		}
		notFoundError.send(w)
		return
	}

	// Log job status without payload to avoid filling up log buffer
	logEvent := logging.Logger().Info().
		Str("job_id", jobID).
		Str("status", jobStatus)
	if jobInfo != nil {
		if ct, ok := jobInfo["circuit_type"]; ok {
			logEvent = logEvent.Interface("circuit_type", ct)
		}
		if ca, ok := jobInfo["created_at"]; ok {
			logEvent = logEvent.Interface("created_at", ca)
		}
	}
	logEvent.Msg("Job found but not completed")

	response := map[string]interface{}{
		"job_id": jobID,
		"status": jobStatus,
	}

	// Handle completed jobs - include result if available
	if jobStatus == "completed" && jobInfo != nil {
		if result, ok := jobInfo["result"]; ok {
			response["result"] = result
			logging.Logger().Info().
				Str("job_id", jobID).
				Msg("Returning result from checkJobExistsDetailed")
		}
	}

	// Handle failed jobs specially - extract actual error details
	if jobStatus == "failed" && jobInfo != nil {
		if payloadRaw, ok := jobInfo["payload"]; ok {
			if payloadStr, ok := payloadRaw.(string); ok {
				var failureDetails map[string]interface{}
				if err := json.Unmarshal([]byte(payloadStr), &failureDetails); err == nil {
					if errorMsg, ok := failureDetails["error"].(string); ok {
						response["message"] = fmt.Sprintf("Job processing failed: %s", errorMsg)
						response["error"] = errorMsg
					}
					if failedAt, ok := failureDetails["failed_at"]; ok {
						response["failed_at"] = failedAt
					}
					if originalJob, ok := failureDetails["original_job"].(map[string]interface{}); ok {
						if circuitType, ok := originalJob["circuit_type"]; ok {
							response["circuit_type"] = circuitType
						}
					}
				} else {
					response["message"] = "Job processing failed. Unable to parse failure details."
				}
			} else {
				response["message"] = "Job processing failed. Unable to access failure details."
			}
		} else {
			response["message"] = "Job processing failed. No failure details available."
		}
	} else {
		// Use generic message for non-failed jobs
		response["message"] = getStatusMessage(jobStatus)

		if jobInfo != nil {
			if createdAt, ok := jobInfo["created_at"]; ok {
				response["created_at"] = createdAt
			}
			if circuitType, ok := jobInfo["circuit_type"]; ok {
				response["circuit_type"] = circuitType
			}
		}
	}

	w.Header().Set("Content-Type", "application/json")

	// Return 200 OK if job is completed with result, otherwise 202 Accepted
	if jobStatus == "completed" {
		if _, hasResult := response["result"]; hasResult {
			w.WriteHeader(http.StatusOK)
		} else {
			w.WriteHeader(http.StatusAccepted)
		}
	} else {
		w.WriteHeader(http.StatusAccepted)
	}

	err = json.NewEncoder(w).Encode(response)
	if err != nil {
		return
	}
}

func isValidJobID(jobID string) bool {
	_, err := uuid.Parse(jobID)
	return err == nil
}

func getStatusMessage(status string) string {
	switch status {
	case "queued":
		return "Job is queued and waiting to be processed"
	case "processing":
		return "Job is currently being processed"
	case "failed":
		return "Job processing failed. Check the failed queue for details"
	case "completed":
		return "Job completed successfully"
	default:
		return "Job status unknown"
	}
}

func (handler proofStatusHandler) checkJobExistsDetailed(jobID string) (bool, string, map[string]interface{}) {
	// First check result cache (fast O(1) lookup)
	result, err := handler.redisQueue.GetResult(jobID)
	if err == nil && result != nil {
		logging.Logger().Debug().
			Str("job_id", jobID).
			Msg("Job found in result cache")

		jobInfo := map[string]interface{}{
			"result":        result,
			"result_cached": true,
		}
		return true, "completed", jobInfo
	}

	// Check job metadata to determine which queue to search (avoids full scan of all queues)
	jobMeta, metaErr := handler.redisQueue.GetJobMeta(jobID)
	if metaErr == nil && jobMeta != nil {
		// We have metadata - check only the relevant queues based on queue name
		if queueName, ok := jobMeta["queue"].(string); ok {
			// Check main queue first
			if job, found := handler.findJobInQueue(queueName, jobID); found {
				return true, "queued", job
			}
			// Check processing queue for this circuit type
			processingQueue := queueName[:len(queueName)-6] + "_processing_queue"
			if job, found := handler.findJobInQueue(processingQueue, jobID); found {
				return true, "processing", job
			}
		}
		// Job has metadata but not found in expected queues - may be in results or failed
		if job, found := handler.findJobInQueue("zk_failed_queue", jobID); found {
			return true, "failed", job
		}
		// Check results queue
		if job, found := handler.findJobInQueue("zk_results_queue", jobID); found {
			if payloadRaw, ok := job["payload"]; ok {
				if payloadStr, ok := payloadRaw.(string); ok {
					var payloadData map[string]interface{}
					if json.Unmarshal([]byte(payloadStr), &payloadData) == nil {
						job["result"] = payloadData
					}
				}
			}
			return true, "completed", job
		}
		// Return metadata-based status even if not found in queues
		// This handles race conditions where job moved between queues
		status := "queued"
		if metaStatus, ok := jobMeta["status"].(string); ok {
			status = metaStatus
		}
		return true, status, map[string]interface{}{
			"circuit_type": jobMeta["circuit_type"],
			"submitted_at": jobMeta["submitted_at"],
			"from_meta":    true,
		}
	}

	// No metadata - fall back to checking failed and results queues only
	// (These are the terminal states where jobs might exist without metadata)
	if job, found := handler.findJobInQueue("zk_failed_queue", jobID); found {
		return true, "failed", job
	}

	if job, found := handler.findJobInQueue("zk_results_queue", jobID); found {
		if payloadRaw, ok := job["payload"]; ok {
			if payloadStr, ok := payloadRaw.(string); ok {
				var payloadData map[string]interface{}
				if json.Unmarshal([]byte(payloadStr), &payloadData) == nil {
					job["result"] = payloadData
				}
			}
		}
		return true, "completed", job
	}

	return false, "", nil
}

func (handler proofStatusHandler) findJobInQueue(queueName, jobID string) (map[string]interface{}, bool) {
	items, err := handler.redisQueue.Client.LRange(handler.redisQueue.Ctx, queueName, 0, -1).Result()
	if err != nil {
		logging.Logger().Error().
			Err(err).
			Str("queue", queueName).
			Str("job_id", jobID).
			Msg("Error searching queue")
		return nil, false
	}

	for _, item := range items {
		var job ProofJob
		if json.Unmarshal([]byte(item), &job) == nil {
			if job.ID == jobID ||
				job.ID == jobID+"_processing" ||
				job.ID == jobID+"_failed" {

				jobInfo := map[string]interface{}{
					"created_at": job.CreatedAt,
				}

				// Include payload for all jobs, especially important for failed jobs
				if len(job.Payload) > 0 {
					jobInfo["payload"] = string(job.Payload)

					var meta map[string]interface{}
					if json.Unmarshal(job.Payload, &meta) == nil {
						if circuitType, ok := meta["circuit_type"]; ok {
							jobInfo["circuit_type"] = circuitType
						}
					}
				}

				logging.Logger().Info().
					Str("job_id", jobID).
					Str("queue", queueName).
					Str("found_job_id", job.ID).
					Msg("Job found in queue")

				return jobInfo, true
			}
		}
	}

	return nil, false
}

type QueueConfig struct {
	RedisURL string
	Enabled  bool
}

type EnhancedConfig struct {
	ProverAddress  string
	MetricsAddress string
	Queue          *QueueConfig
}

type proveHandler struct {
	keyManager  *common.LazyKeyManager
	redisQueue  *RedisQueue
	enableQueue bool
}

func (handler proveHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}

	buf, err := io.ReadAll(r.Body)
	if err != nil {
		logging.Logger().Error().Err(err).Msg("Error reading request body")
		malformedBodyError(err).send(w)
		return
	}

	proofRequestMeta, err := common.ParseProofRequestMeta(buf)
	if err != nil {
		malformedBodyError(err).send(w)
		return
	}

	forceAsync := r.Header.Get("X-Async") == "true" || r.URL.Query().Get("async") == "true"
	forceSync := r.Header.Get("X-Sync") == "true" || r.URL.Query().Get("sync") == "true"

	shouldUseQueue := handler.shouldUseQueueForCircuit(proofRequestMeta.CircuitType, forceAsync, forceSync)

	logging.Logger().Info().
		Str("circuit_type", string(proofRequestMeta.CircuitType)).
		Bool("force_async", forceAsync).
		Bool("force_sync", forceSync).
		Bool("use_queue", shouldUseQueue).
		Bool("queue_available", handler.enableQueue && handler.redisQueue != nil).
		Msg("Processing prove request")

	if shouldUseQueue && handler.enableQueue && handler.redisQueue != nil {
		handler.handleAsyncProof(w, r, buf, proofRequestMeta)
	} else {
		handler.handleSyncProof(w, r, buf, proofRequestMeta)
	}
}

func (handler proveHandler) shouldUseQueueForCircuit(circuitType common.CircuitType, forceAsync, forceSync bool) bool {
	if !handler.enableQueue || handler.redisQueue == nil {
		return false
	}

	// Always use queue for batch operations when queue is available
	// This prevents cross-contamination in clustered deployments
	if circuitType == common.BatchUpdateCircuitType ||
		circuitType == common.BatchAppendCircuitType ||
		circuitType == common.BatchAddressAppendCircuitType {
		return true
	}

	// For non-batch operations, respect sync/async preferences
	if forceAsync {
		return true
	}
	if forceSync {
		return false
	}

	// Non-batch operations default to local processing
	return false
}

type queueStatsHandler struct {
	redisQueue *RedisQueue
}

func (handler queueStatsHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}

	stats, err := handler.redisQueue.GetQueueStats()
	if err != nil {
		unexpectedError(err).send(w)
		return
	}

	response := map[string]interface{}{
		"queues":        stats,
		"total_pending": stats["zk_update_queue"] + stats["zk_append_queue"] + stats["zk_address_append_queue"],
		"total_active":  stats["zk_update_processing_queue"] + stats["zk_append_processing_queue"] + stats["zk_address_append_processing_queue"],
		"total_failed":  stats["zk_failed_queue"],
		"timestamp":     time.Now().Unix(),
	}

	w.Header().Set("Content-Type", "application/json")
	err = json.NewEncoder(w).Encode(response)
	if err != nil {
		return
	}
}

type queueHealthHandler struct {
	redisQueue *RedisQueue
}

func (handler queueHealthHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}

	health, err := handler.redisQueue.GetQueueHealth()
	if err != nil {
		unexpectedError(err).send(w)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	err = json.NewEncoder(w).Encode(health)
	if err != nil {
		return
	}
}

type queueCleanupHandler struct {
	redisQueue *RedisQueue
}

func (handler queueCleanupHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}

	results := make(map[string]interface{})

	if err := handler.redisQueue.CleanupOldRequests(); err != nil {
		results["old_requests_cleanup"] = map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		}
	} else {
		results["old_requests_cleanup"] = map[string]interface{}{
			"success": true,
		}
	}

	if err := handler.redisQueue.CleanupStuckProcessingJobs(); err != nil {
		results["stuck_jobs_cleanup"] = map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		}
	} else {
		results["stuck_jobs_cleanup"] = map[string]interface{}{
			"success": true,
		}
	}

	if err := handler.redisQueue.CleanupOldFailedJobs(); err != nil {
		results["old_failed_cleanup"] = map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		}
	} else {
		results["old_failed_cleanup"] = map[string]interface{}{
			"success": true,
		}
	}

	if err := handler.redisQueue.CleanupOldResultKeys(); err != nil {
		results["old_result_keys_cleanup"] = map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		}
	} else {
		results["old_result_keys_cleanup"] = map[string]interface{}{
			"success": true,
		}
	}

	results["timestamp"] = time.Now().Unix()

	w.Header().Set("Content-Type", "application/json")
	err := json.NewEncoder(w).Encode(results)
	if err != nil {
		return
	}
}

func RunWithQueue(config *Config, redisQueue *RedisQueue, keyManager *common.LazyKeyManager) RunningJob {
	return RunEnhanced(&EnhancedConfig{
		ProverAddress:  config.ProverAddress,
		MetricsAddress: config.MetricsAddress,
		Queue: &QueueConfig{
			Enabled: redisQueue != nil,
		},
	}, redisQueue, keyManager)
}

func RunEnhanced(config *EnhancedConfig, redisQueue *RedisQueue, keyManager *common.LazyKeyManager) RunningJob {
	apiKey := getAPIKeyFromEnv()
	if apiKey != "" {
		logging.Logger().Info().Msg("API key authentication enabled for prover server")
	} else {
		logging.Logger().Warn().Msg("No API key configured - server will accept all requests. Set PROVER_API_KEY environment variable to enable authentication.")
	}
	metricsMux := http.NewServeMux()
	metricsMux.Handle("/metrics", promhttp.Handler())
	metricsServer := &http.Server{Addr: config.MetricsAddress, Handler: metricsMux}
	metricsJob := spawnServerJob(metricsServer, "metrics server")
	logging.Logger().Info().Str("addr", config.MetricsAddress).Msg("metrics server started")

	proverMux := http.NewServeMux()

	proverMux.Handle("/prove", proveHandler{
		keyManager:  keyManager,
		redisQueue:  redisQueue,
		enableQueue: config.Queue != nil && config.Queue.Enabled,
	})

	proverMux.Handle("/health", healthHandler{})

	if redisQueue != nil {
		proverMux.Handle("/prove/status", proofStatusHandler{redisQueue: redisQueue})
		proverMux.Handle("/queue/stats", queueStatsHandler{redisQueue: redisQueue})
		proverMux.Handle("/queue/health", queueHealthHandler{redisQueue: redisQueue})
		proverMux.Handle("/queue/cleanup", queueCleanupHandler{redisQueue: redisQueue})

		proverMux.HandleFunc("/queue/add", func(w http.ResponseWriter, r *http.Request) {
			if r.Method != http.MethodPost {
				w.WriteHeader(http.StatusMethodNotAllowed)
				return
			}

			buf, err := io.ReadAll(r.Body)
			if err != nil {
				malformedBodyError(err).send(w)
				return
			}

			proofRequestMeta, err := common.ParseProofRequestMeta(buf)
			if err != nil {
				malformedBodyError(err).send(w)
				return
			}

			queueName := GetQueueNameForCircuit(proofRequestMeta.CircuitType)

			// Compute input hash for deduplication
			inputHash := ComputeInputHash(json.RawMessage(buf))

			// Check if there's already an in-flight job with the same input
			jobID := uuid.New().String()
			existingJobID, isNew, err := redisQueue.GetOrSetInFlightJob(inputHash, jobID)
			if err != nil {
				logging.Logger().Warn().
					Err(err).
					Str("input_hash", inputHash).
					Msg("Failed to check for in-flight job, proceeding with new job")
				existingJobID = jobID
				isNew = true
			}

			// If there's already an in-flight job, verify it exists before returning its ID
			if !isNew {
				// Verify the deduplicated job actually exists (has result, metadata, or is in queue)
				jobExists := false
				if result, _ := redisQueue.GetResult(existingJobID); result != nil {
					jobExists = true
				} else if meta, _ := redisQueue.GetJobMeta(existingJobID); meta != nil {
					jobExists = true
				}

				if jobExists {
					response := map[string]interface{}{
						"job_id":       existingJobID,
						"status":       "already_queued",
						"queue":        queueName,
						"circuit_type": string(proofRequestMeta.CircuitType),
						"message":      "Proof request with identical input already in queue. Returning existing job ID.",
						"deduplicated": true,
					}

					logging.Logger().Info().
						Str("existing_job_id", existingJobID).
						Str("input_hash", inputHash).
						Str("circuit_type", string(proofRequestMeta.CircuitType)).
						Msg("Deduplicated proof request via /queue/add")

					w.Header().Set("Content-Type", "application/json")
					w.WriteHeader(http.StatusAccepted)
					json.NewEncoder(w).Encode(response)
					return
				}

				// Job doesn't exist - clean up stale marker and create new job
				logging.Logger().Warn().
					Str("stale_job_id", existingJobID).
					Str("input_hash", inputHash).
					Msg("Deduplication found stale job ID - cleaning up and creating new job")
				redisQueue.CleanupStaleInFlightMarker(existingJobID)
				// Generate new job ID and set in-flight marker
				newJobID := uuid.New().String()
				redisQueue.Client.Set(redisQueue.Ctx, fmt.Sprintf("zk_inflight_%s", inputHash), newJobID, 10*time.Minute)
				existingJobID = newJobID // Update so line below assigns correct ID
				isNew = true
			}

			// This is a new job
			jobID = existingJobID

			job := &ProofJob{
				ID:         jobID,
				Type:       "zk_proof",
				Payload:    json.RawMessage(buf),
				CreatedAt:  time.Now(),
				TreeID:     proofRequestMeta.TreeID,
				BatchIndex: proofRequestMeta.BatchIndex,
			}

			// Store job metadata BEFORE enqueueing to prevent race condition where worker
			// picks up job before metadata exists, causing job_not_found on status checks
			if err := redisQueue.StoreJobMeta(jobID, queueName, string(proofRequestMeta.CircuitType)); err != nil {
				logging.Logger().Warn().
					Err(err).
					Str("job_id", jobID).
					Str("queue", queueName).
					Msg("Failed to store job metadata (will still attempt to enqueue)")
			}

			// Store input hash mapping for cleanup when job completes
			redisQueue.StoreInputHash(jobID, inputHash)

			err = redisQueue.EnqueueProof(queueName, job)
			if err != nil {
				// Clean up in-flight marker and metadata since we failed to enqueue
				redisQueue.DeleteInFlightJob(inputHash, jobID)
				redisQueue.DeleteJobMeta(jobID)
				unexpectedError(err).send(w)
				return
			}

			logging.Logger().Info().
				Str("job_id", jobID).
				Str("queue", queueName).
				Str("circuit_type", string(proofRequestMeta.CircuitType)).
				Msg("Enqueued proof job")

			response := map[string]interface{}{
				"job_id":       jobID,
				"status":       "queued",
				"queue":        queueName,
				"circuit_type": string(proofRequestMeta.CircuitType),
				"message":      fmt.Sprintf("Job queued in %s", queueName),
			}

			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusAccepted)
			err = json.NewEncoder(w).Encode(response)
			if err != nil {
				return
			}
		})
	}

	corsHandler := handlers.CORS(
		handlers.AllowedHeaders([]string{
			"X-Requested-With",
			"Content-Type",
			"Authorization",
			"X-API-Key",
			"X-Async",
			"X-Sync",
		}),
		handlers.AllowedOrigins([]string{"*"}),
		handlers.AllowedMethods([]string{"GET", "POST", "PUT", "DELETE", "OPTIONS"}),
	)

	authHandler := conditionalAuthMiddleware(apiKey)
	proverServer := &http.Server{Addr: config.ProverAddress, Handler: corsHandler(authHandler(proverMux))}
	proverJob := spawnServerJob(proverServer, "prover server")

	if redisQueue != nil {
		logging.Logger().Info().
			Str("addr", config.ProverAddress).
			Bool("queue_enabled", true).
			Msg("enhanced prover server started with Redis queue support")
	} else {
		logging.Logger().Info().
			Str("addr", config.ProverAddress).
			Bool("queue_enabled", false).
			Msg("prover server started (no queue support)")
	}

	return CombineJobs(metricsJob, proverJob)
}

func Run(config *Config, keyManager *common.LazyKeyManager) RunningJob {
	return RunWithQueue(config, nil, keyManager)
}

type Error struct {
	StatusCode int
	Code       string
	Message    string
}

func malformedBodyError(err error) *Error {
	return &Error{StatusCode: http.StatusBadRequest, Code: "malformed_body", Message: err.Error()}
}

func provingError(err error) *Error {
	return &Error{StatusCode: http.StatusBadRequest, Code: "proving_error", Message: err.Error()}
}

func unexpectedError(err error) *Error {
	return &Error{StatusCode: http.StatusInternalServerError, Code: "unexpected_error", Message: err.Error()}
}

func (error *Error) MarshalJSON() ([]byte, error) {
	return json.Marshal(map[string]string{
		"code":    error.Code,
		"message": error.Message,
	})
}

func (error *Error) send(w http.ResponseWriter) {
	w.WriteHeader(error.StatusCode)
	jsonBytes, err := error.MarshalJSON()
	if err != nil {
		jsonBytes = []byte(`{"code": "unexpected_error", "message": "failed to marshal error"}`)
	}
	length, err := w.Write(jsonBytes)
	if err != nil || length != len(jsonBytes) {
		logging.Logger().Error().Err(err).Msg("error writing response")
	}
}

type Config struct {
	ProverAddress  string
	MetricsAddress string
}

func spawnServerJob(server *http.Server, label string) RunningJob {
	start := func() {
		err := server.ListenAndServe()
		if err != nil && !errors.Is(err, http.ErrServerClosed) {
			panic(fmt.Sprintf("%s failed: %s", label, err))
		}
	}
	shutdown := func() {
		logging.Logger().Info().Msgf("shutting down %s", label)
		err := server.Shutdown(context.Background())
		if err != nil {
			logging.Logger().Error().Err(err).Msgf("error when shutting down %s", label)
		}
		logging.Logger().Info().Msgf("%s shut down", label)
	}
	return SpawnJob(start, shutdown)
}

type healthHandler struct {
}

func (handler proveHandler) handleAsyncProof(w http.ResponseWriter, r *http.Request, buf []byte, meta common.ProofRequestMeta) {
	ProofRequestsTotal.WithLabelValues(string(meta.CircuitType)).Inc()
	RecordCircuitInputSize(string(meta.CircuitType), len(buf))

	queueName := GetQueueNameForCircuit(meta.CircuitType)

	// Compute input hash for deduplication
	inputHash := ComputeInputHash(json.RawMessage(buf))

	// Check if there's already an in-flight job with the same input
	jobID := uuid.New().String()
	existingJobID, isNew, err := handler.redisQueue.GetOrSetInFlightJob(inputHash, jobID)
	if err != nil {
		logging.Logger().Warn().
			Err(err).
			Str("input_hash", inputHash).
			Msg("Failed to check for in-flight job, proceeding with new job")
		// Continue with new job on error
		existingJobID = jobID
		isNew = true
	}

	// If there's already an in-flight job, verify it exists before returning its ID
	if !isNew {
		// Verify the deduplicated job actually exists (has result, metadata, or is in queue)
		jobExists := false
		if result, _ := handler.redisQueue.GetResult(existingJobID); result != nil {
			jobExists = true
		} else if jobMeta, _ := handler.redisQueue.GetJobMeta(existingJobID); jobMeta != nil {
			jobExists = true
		}

		if jobExists {
			estimatedTime := handler.getEstimatedTime(meta.CircuitType)

			response := map[string]interface{}{
				"job_id":         existingJobID,
				"status":         "already_queued",
				"circuit_type":   string(meta.CircuitType),
				"queue":          queueName,
				"estimated_time": estimatedTime,
				"status_url":     fmt.Sprintf("/prove/status?job_id=%s", existingJobID),
				"message":        "Proof request with identical input already in queue. Returning existing job ID.",
				"deduplicated":   true,
			}

			logging.Logger().Info().
				Str("existing_job_id", existingJobID).
				Str("input_hash", inputHash).
				Str("circuit_type", string(meta.CircuitType)).
				Msg("Deduplicated proof request - returning existing job")

			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusAccepted)
			json.NewEncoder(w).Encode(response)
			return
		}

		// Job doesn't exist - clean up stale marker and create new job
		logging.Logger().Warn().
			Str("stale_job_id", existingJobID).
			Str("input_hash", inputHash).
			Msg("Deduplication found stale job ID - cleaning up and creating new job")
		handler.redisQueue.CleanupStaleInFlightMarker(existingJobID)
		// Generate new job ID and set in-flight marker
		newJobID := uuid.New().String()
		handler.redisQueue.Client.Set(handler.redisQueue.Ctx, fmt.Sprintf("zk_inflight_%s", inputHash), newJobID, 10*time.Minute)
		existingJobID = newJobID // Update so line below assigns correct ID
		isNew = true
	}

	// This is a new job - use the job ID we registered
	jobID = existingJobID

	job := &ProofJob{
		ID:         jobID,
		Type:       "zk_proof",
		Payload:    json.RawMessage(buf),
		CreatedAt:  time.Now(),
		TreeID:     meta.TreeID,
		BatchIndex: meta.BatchIndex,
	}

	// Store job metadata BEFORE enqueueing to prevent race condition where worker
	// picks up job before metadata exists, causing job_not_found on status checks
	if err := handler.redisQueue.StoreJobMeta(jobID, queueName, string(meta.CircuitType)); err != nil {
		logging.Logger().Warn().
			Err(err).
			Str("job_id", jobID).
			Str("queue", queueName).
			Msg("Failed to store job metadata (will still attempt to enqueue)")
	}

	// Store input hash mapping for cleanup when job completes
	handler.redisQueue.StoreInputHash(jobID, inputHash)

	err = handler.redisQueue.EnqueueProof(queueName, job)
	if err != nil {
		logging.Logger().Error().Err(err).Msg("Failed to enqueue proof job")

		// Clean up in-flight marker and metadata since we failed to enqueue
		if delErr := handler.redisQueue.DeleteInFlightJob(inputHash, jobID); delErr != nil {
			logging.Logger().Error().Err(delErr).Str("job_id", jobID).Msg("Failed to cleanup in-flight marker after enqueue failure - may cause stale deduplication")
		}
		if delErr := handler.redisQueue.DeleteJobMeta(jobID); delErr != nil {
			logging.Logger().Error().Err(delErr).Str("job_id", jobID).Msg("Failed to cleanup job metadata after enqueue failure")
		}

		if handler.isBatchOperation(meta.CircuitType) {
			serviceUnavailableError := &Error{
				StatusCode: http.StatusServiceUnavailable,
				Code:       "queue_unavailable",
				Message:    fmt.Sprintf("Queue service unavailable and %s requires asynchronous processing", meta.CircuitType),
			}
			serviceUnavailableError.send(w)
			return
		}

		logging.Logger().Warn().Msg("Queue failed, falling back to synchronous processing")
		handler.handleSyncProof(w, r, buf, meta)
		return
	}

	estimatedTime := handler.getEstimatedTime(meta.CircuitType)

	response := map[string]interface{}{
		"job_id":         jobID,
		"status":         "queued",
		"circuit_type":   string(meta.CircuitType),
		"queue":          queueName,
		"estimated_time": estimatedTime,
		"status_url":     fmt.Sprintf("/prove/status?job_id=%s", jobID),
		"message":        fmt.Sprintf("Proof generation queued for %s circuit. Use status_url to check progress.", meta.CircuitType),
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusAccepted)
	err = json.NewEncoder(w).Encode(response)
	if err != nil {
		return
	}

	logging.Logger().Info().
		Str("job_id", jobID).
		Str("queue", queueName).
		Str("circuit_type", string(meta.CircuitType)).
		Msg("Batch operation job queued successfully")
}

func (handler proveHandler) handleSyncProof(w http.ResponseWriter, r *http.Request, buf []byte, meta common.ProofRequestMeta) {
	if handler.isBatchOperation(meta.CircuitType) {
		warning := fmt.Sprintf("WARNING: %s is a heavy operation that should be processed asynchronously. Consider using X-Async: true header.", meta.CircuitType)
		w.Header().Set("X-Warning", warning)
		logging.Logger().Warn().
			Str("circuit_type", string(meta.CircuitType)).
			Msg("Processing batch operation synchronously - this may cause timeouts")
	}

	estimatedTime := handler.getEstimatedTimeSeconds(meta.CircuitType)
	timeoutDuration := time.Duration(estimatedTime*2) * time.Second
	if timeoutDuration < 10*time.Second {
		timeoutDuration = 10 * time.Second
	}
	if timeoutDuration > 300*time.Second {
		timeoutDuration = 300 * time.Second
	}

	ctx, cancel := context.WithTimeout(r.Context(), timeoutDuration)
	defer cancel()

	type proofResult struct {
		proof *common.Proof
		err   *Error
	}

	resultChan := make(chan proofResult, 1)

	go func() {
		// Recover from panics to prevent server crash from malformed input
		defer func() {
			if r := recover(); r != nil {
				logging.Logger().Error().
					Interface("panic", r).
					Str("circuit_type", string(meta.CircuitType)).
					Msg("Panic recovered in proof processing")
				resultChan <- proofResult{
					proof: nil,
					err:   unexpectedError(fmt.Errorf("internal error during proof processing: %v", r)),
				}
			}
		}()

		timer := StartProofTimer(string(meta.CircuitType))
		RecordCircuitInputSize(string(meta.CircuitType), len(buf))

		proof, proofError := handler.processProofSync(buf)

		if proofError != nil {
			timer.ObserveError(proofError.Code)
			RecordJobComplete(false)
		} else {
			timer.ObserveDuration()
			RecordJobComplete(true)
			if proof != nil {
				proofBytes, _ := json.Marshal(proof)
				RecordProofSize(string(meta.CircuitType), len(proofBytes))
			}
		}

		resultChan <- proofResult{proof: proof, err: proofError}
	}()

	select {
	case result := <-resultChan:
		if result.err != nil {
			result.err.send(w)
			return
		}

		responseBytes, err := json.Marshal(result.proof)
		if err != nil {
			unexpectedError(err).send(w)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, err = w.Write(responseBytes)
		if err != nil {
			return
		}

		logging.Logger().Info().
			Str("circuit_type", string(meta.CircuitType)).
			Msg("Synchronous proof completed successfully")

	case <-ctx.Done():
		timeoutError := &Error{
			StatusCode: http.StatusRequestTimeout,
			Code:       "proof_timeout",
			Message:    fmt.Sprintf("Proof generation timed out after %d seconds. For %s circuits, use asynchronous mode with X-Async: true header.", int(timeoutDuration.Seconds()), meta.CircuitType),
		}
		timeoutError.send(w)

		logging.Logger().Warn().
			Str("circuit_type", string(meta.CircuitType)).
			Int("timeout_seconds", int(timeoutDuration.Seconds())).
			Msg("Synchronous proof timed out")
	}
}

func (handler proveHandler) isBatchOperation(circuitType common.CircuitType) bool {
	switch circuitType {
	case common.BatchAppendCircuitType,
		common.BatchUpdateCircuitType,
		common.BatchAddressAppendCircuitType:
		return true
	default:
		return false
	}
}

func GetQueueNameForCircuit(circuitType common.CircuitType) string {
	switch circuitType {
	case common.BatchUpdateCircuitType:
		return "zk_update_queue"
	case common.BatchAppendCircuitType:
		return "zk_append_queue"
	case common.BatchAddressAppendCircuitType:
		return "zk_address_append_queue"
	default:
		return "zk_update_queue"
	}
}

func (handler proveHandler) getEstimatedTime(circuitType common.CircuitType) string {
	switch circuitType {
	case common.InclusionCircuitType:
		return "1-3 seconds"
	case common.NonInclusionCircuitType:
		return "1-3 seconds"
	case common.CombinedCircuitType:
		return "1-3 seconds"
	case common.BatchAppendCircuitType:
		return "10-30 seconds"
	case common.BatchUpdateCircuitType:
		return "10-30 seconds"
	case common.BatchAddressAppendCircuitType:
		return "10-30 seconds"
	default:
		return "1-3 seconds"
	}
}

func (handler proveHandler) getEstimatedTimeSeconds(circuitType common.CircuitType) int {
	switch circuitType {
	case common.InclusionCircuitType:
		return 1
	case common.NonInclusionCircuitType:
		return 1
	case common.CombinedCircuitType:
		return 1
	case common.BatchAppendCircuitType:
		return 30
	case common.BatchUpdateCircuitType:
		return 30
	case common.BatchAddressAppendCircuitType:
		return 30
	default:
		return 1
	}
}

func (handler proveHandler) processProofSync(buf []byte) (*common.Proof, *Error) {
	proofRequestMeta, err := common.ParseProofRequestMeta(buf)
	if err != nil {
		return nil, malformedBodyError(err)
	}

	switch proofRequestMeta.CircuitType {
	case common.InclusionCircuitType:
		return handler.inclusionProof(buf, proofRequestMeta)
	case common.NonInclusionCircuitType:
		return handler.nonInclusionProof(buf, proofRequestMeta)
	case common.CombinedCircuitType:
		return handler.combinedProof(buf, proofRequestMeta)
	case common.BatchUpdateCircuitType:
		return handler.batchUpdateProof(buf)
	case common.BatchAppendCircuitType:
		return handler.batchAppendHandler(buf)
	case common.BatchAddressAppendCircuitType:
		return handler.batchAddressAppendProof(buf)
	default:
		return nil, malformedBodyError(fmt.Errorf("unknown circuit type: %s", proofRequestMeta.CircuitType))
	}
}

func (handler proveHandler) batchAddressAppendProof(buf []byte) (*common.Proof, *Error) {
	var params v2.BatchAddressAppendParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)
	}

	treeHeight := params.TreeHeight
	batchSize := params.BatchSize

	ps, err := handler.keyManager.GetBatchSystem(common.BatchAddressAppendCircuitType, treeHeight, batchSize)
	if err != nil {
		return nil, provingError(fmt.Errorf("batch address append: %w", err))
	}

	proof, err := v2.ProveBatchAddressAppend(ps, &params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) batchAppendHandler(buf []byte) (*common.Proof, *Error) {
	var params v2.BatchAppendParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		return nil, malformedBodyError(err)
	}

	treeHeight := params.Height
	batchSize := params.BatchSize

	ps, err := handler.keyManager.GetBatchSystem(common.BatchAppendCircuitType, treeHeight, batchSize)
	if err != nil {
		return nil, provingError(fmt.Errorf("batch append: %w", err))
	}

	proof, err := v2.ProveBatchAppend(ps, &params)
	if err != nil {
		logging.Logger().Err(err).Msg("Error during proof generation")
		return nil, provingError(err)
	}

	return proof, nil
}

func (handler proveHandler) batchUpdateProof(buf []byte) (*common.Proof, *Error) {
	var params v2.BatchUpdateParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		return nil, malformedBodyError(err)
	}

	treeHeight := params.Height
	batchSize := params.BatchSize

	ps, err := handler.keyManager.GetBatchSystem(common.BatchUpdateCircuitType, treeHeight, batchSize)
	if err != nil {
		return nil, provingError(fmt.Errorf("batch update: %w", err))
	}

	proof, err := v2.ProveBatchUpdate(ps, &params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) inclusionProof(buf []byte, proofRequestMeta common.ProofRequestMeta) (*common.Proof, *Error) {
	ps, err := handler.keyManager.GetMerkleSystem(
		proofRequestMeta.StateTreeHeight,
		proofRequestMeta.NumInputs,
		0,
		0,
		proofRequestMeta.Version,
	)
	if err != nil {
		return nil, provingError(fmt.Errorf("inclusion proof: %w", err))
	}

	switch proofRequestMeta.Version {
	case 1:
		var params v1.InclusionParameters

		if err := json.Unmarshal(buf, &params); err != nil {
			return nil, malformedBodyError(err)
		}
		proof, err := v1.ProveInclusion(ps, &params)
		if err != nil {
			return nil, provingError(err)
		}
		return proof, nil
	case 2:
		var params v2.InclusionParameters
		if err := json.Unmarshal(buf, &params); err != nil {
			return nil, malformedBodyError(err)
		}
		proof, err := v2.ProveInclusion(ps, &params)
		if err != nil {
			return nil, provingError(err)
		}
		return proof, nil
	}

	return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
}

func (handler proveHandler) nonInclusionProof(buf []byte, proofRequestMeta common.ProofRequestMeta) (*common.Proof, *Error) {
	version := uint32(1)
	if proofRequestMeta.AddressTreeHeight == 40 {
		version = 2
	}

	ps, err := handler.keyManager.GetMerkleSystem(
		0,
		0,
		proofRequestMeta.AddressTreeHeight,
		proofRequestMeta.NumAddresses,
		version,
	)
	if err != nil {
		return nil, provingError(fmt.Errorf("non-inclusion proof: %w", err))
	}

	switch proofRequestMeta.AddressTreeHeight {
	case 26:
		var params v1.NonInclusionParameters

		var err = json.Unmarshal(buf, &params)
		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err := v1.ProveNonInclusion(ps, &params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	case 40:
		var params v2.NonInclusionParameters

		var err = json.Unmarshal(buf, &params)
		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err := v2.ProveNonInclusion(ps, &params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	default:
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
}

func (handler proveHandler) combinedProof(buf []byte, proofRequestMeta common.ProofRequestMeta) (*common.Proof, *Error) {
	version := uint32(1)
	if proofRequestMeta.AddressTreeHeight == 40 {
		version = 2
	}

	ps, err := handler.keyManager.GetMerkleSystem(
		proofRequestMeta.StateTreeHeight,
		proofRequestMeta.NumInputs,
		proofRequestMeta.AddressTreeHeight,
		proofRequestMeta.NumAddresses,
		version,
	)
	if err != nil {
		return nil, provingError(fmt.Errorf("combined proof: %w", err))
	}

	switch proofRequestMeta.AddressTreeHeight {
	case 26:
		var params v1.CombinedParameters
		if err := json.Unmarshal(buf, &params); err != nil {
			return nil, malformedBodyError(err)
		}
		proof, err := v1.ProveCombined(ps, &params)
		if err != nil {
			return nil, provingError(err)
		}
		return proof, nil
	case 40:
		var params v2.CombinedParameters
		if err := json.Unmarshal(buf, &params); err != nil {
			return nil, malformedBodyError(err)
		}
		proof, err := v2.ProveCombined(ps, &params)
		if err != nil {
			return nil, provingError(err)
		}
		return proof, nil
	default:
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
}

func (handler healthHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	logging.Logger().Info().Msg("received health check request")
	responseBytes, err := json.Marshal(map[string]string{"status": "ok"})
	if err != nil {
		logging.Logger().Error().Err(err).Msg("error marshaling response")
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
	_, err = w.Write(responseBytes)
	if err != nil {
		logging.Logger().Error().Err(err).Msg("error writing response")
	}
}
