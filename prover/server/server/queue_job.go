package server

import (
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	v1 "light/light-prover/prover/v1"
	v2 "light/light-prover/prover/v2"
	"log"
	"os"
	"strconv"
	"time"
)

const (
	// JobExpirationTimeout should match the forester's max_wait_time (600 seconds)
	JobExpirationTimeout = 600 * time.Second
	// DefaultMaxConcurrency is the default number of proofs to process in parallel per queue
	DefaultMaxConcurrency = 200
)

// getMaxConcurrency reads the PROVER_MAX_CONCURRENCY environment variable
// or returns the default value
func getMaxConcurrency() int {
	if val := os.Getenv("PROVER_MAX_CONCURRENCY"); val != "" {
		if concurrency, err := strconv.Atoi(val); err == nil && concurrency > 0 {
			logging.Logger().Info().
				Int("max_concurrency", concurrency).
				Msg("Using custom max concurrency from PROVER_MAX_CONCURRENCY")
			return concurrency
		}
	}
	logging.Logger().Info().
		Int("max_concurrency", DefaultMaxConcurrency).
		Msg("Using default max concurrency")
	return DefaultMaxConcurrency
}

type ProofJob struct {
	ID        string          `json:"id"`
	Type      string          `json:"type"`
	Payload   json.RawMessage `json:"payload"`
	CreatedAt time.Time       `json:"created_at"`
}

type QueueWorker interface {
	Start()
	Stop()
}

type BaseQueueWorker struct {
	queue               *RedisQueue
	keyManager          *common.LazyKeyManager
	stopChan            chan struct{}
	queueName           string
	processingQueueName string
	maxConcurrency      int
	semaphore           chan struct{}
}

type UpdateQueueWorker struct {
	*BaseQueueWorker
}

type AppendQueueWorker struct {
	*BaseQueueWorker
}

type AddressAppendQueueWorker struct {
	*BaseQueueWorker
}

func NewUpdateQueueWorker(redisQueue *RedisQueue, keyManager *common.LazyKeyManager) *UpdateQueueWorker {
	maxConcurrency := getMaxConcurrency()
	return &UpdateQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_update_queue",
			processingQueueName: "zk_update_processing_queue",
			maxConcurrency:      maxConcurrency,
			semaphore:           make(chan struct{}, maxConcurrency),
		},
	}
}

func NewAppendQueueWorker(redisQueue *RedisQueue, keyManager *common.LazyKeyManager) *AppendQueueWorker {
	maxConcurrency := getMaxConcurrency()
	return &AppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_append_queue",
			processingQueueName: "zk_append_processing_queue",
			maxConcurrency:      maxConcurrency,
			semaphore:           make(chan struct{}, maxConcurrency),
		},
	}
}

func NewAddressAppendQueueWorker(redisQueue *RedisQueue, keyManager *common.LazyKeyManager) *AddressAppendQueueWorker {
	maxConcurrency := getMaxConcurrency()
	return &AddressAppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_address_append_queue",
			processingQueueName: "zk_address_append_processing_queue",
			maxConcurrency:      maxConcurrency,
			semaphore:           make(chan struct{}, maxConcurrency),
		},
	}
}

func (w *BaseQueueWorker) Start() {
	logging.Logger().Info().
		Str("queue", w.queueName).
		Int("max_concurrency", w.maxConcurrency).
		Msg("Starting queue worker with parallel processing")

	for {
		select {
		case <-w.stopChan:
			logging.Logger().Info().Str("queue", w.queueName).Msg("Queue worker stopping")
			return
		default:
			w.processJobs()
		}
	}
}

func (w *BaseQueueWorker) Stop() {
	close(w.stopChan)
}

func (w *BaseQueueWorker) processJobs() {
	job, err := w.queue.DequeueProof(w.queueName, 5*time.Second)
	if err != nil {
		logging.Logger().Error().Err(err).Str("queue", w.queueName).Msg("Error dequeuing from queue")
		time.Sleep(2 * time.Second)
		return
	}

	if job == nil {
		time.Sleep(1 * time.Second)
		return
	}

	// Check if a job has expired
	if !job.CreatedAt.IsZero() {
		jobAge := time.Since(job.CreatedAt)
		if jobAge > JobExpirationTimeout {
			logging.Logger().Warn().
				Str("job_id", job.ID).
				Str("job_type", job.Type).
				Str("queue", w.queueName).
				Dur("job_age", jobAge).
				Dur("expiration_timeout", JobExpirationTimeout).
				Time("created_at", job.CreatedAt).
				Msg("Skipping expired job - forester likely timed out")

			// Record metrics for expired jobs
			ExpiredJobsCounter.WithLabelValues(w.queueName).Inc()

			// Add to failed queue with expiration reason
			expirationErr := fmt.Errorf("job expired after %v (max: %v)", jobAge, JobExpirationTimeout)
			w.addToFailedQueue(job, expirationErr)
			return
		}

		queueWaitTime := jobAge.Seconds()
		circuitType := "unknown"
		switch w.queueName {
		case "zk_update_queue":
			circuitType = "update"
		case "zk_append_queue":
			circuitType = "append"
		case "zk_address_append_queue":
			circuitType = "address-append"
		}
		QueueWaitTime.WithLabelValues(circuitType).Observe(queueWaitTime)
	}

	logging.Logger().Info().
		Str("job_id", job.ID).
		Str("job_type", job.Type).
		Str("queue", w.queueName).
		Msg("Dequeued proof job")

	// Check for duplicate inputs before processing
	inputHash := ComputeInputHash(job.Payload)

	// Check if we already have a successful result for this input
	cachedProof, cachedJobID, err := w.queue.FindCachedResult(inputHash)
	if err != nil {
		logging.Logger().Warn().
			Err(err).
			Str("job_id", job.ID).
			Str("input_hash", inputHash).
			Msg("Error searching for cached result, continuing with processing")
	} else if cachedProof != nil {
		// Found a cached successful result, return it immediately
		logging.Logger().Info().
			Str("job_id", job.ID).
			Str("cached_job_id", cachedJobID).
			Str("input_hash", inputHash).
			Msg("Returning cached successful proof result without re-processing")

		// Store result for new job ID
		resultData, _ := json.Marshal(cachedProof)
		resultJob := &ProofJob{
			ID:        job.ID,
			Type:      "result",
			Payload:   json.RawMessage(resultData),
			CreatedAt: time.Now(),
		}
		err = w.queue.EnqueueProof("zk_results_queue", resultJob)
		if err != nil {
			logging.Logger().Error().Err(err).Str("job_id", job.ID).Msg("Failed to enqueue cached result")
		}
		w.queue.StoreResult(job.ID, cachedProof)
		w.queue.StoreInputHash(job.ID, inputHash)
		return
	}

	// Check if we already have a failure for this input
	// Avoid reusing cached failures for address append to give the forester a chance
	// to recover from transient witness issues and redis inconsistencies.
	if w.queueName != "zk_address_append_queue" {
		cachedFailure, cachedFailedJobID, err := w.queue.FindCachedFailure(inputHash)
		if err != nil {
			logging.Logger().Warn().
				Err(err).
				Str("job_id", job.ID).
				Str("input_hash", inputHash).
				Msg("Error searching for cached failure, continuing with processing")
		} else if cachedFailure != nil {
			// Found a cached failure, return it immediately
			logging.Logger().Info().
				Str("job_id", job.ID).
				Str("cached_job_id", cachedFailedJobID).
				Str("input_hash", inputHash).
				Msg("Returning cached failure without re-processing")

			// Extract error message from cached failure
			var errorMsg string
			if errMsg, ok := cachedFailure["error"].(string); ok {
				errorMsg = errMsg
			} else {
				errorMsg = "Proof generation failed (cached failure)"
			}

			// Add to failed queue with new job ID
			failedJob := map[string]interface{}{
				"original_job": job,
				"error":        errorMsg,
				"failed_at":    time.Now(),
				"cached_from":  cachedFailedJobID,
			}

			failedData, _ := json.Marshal(failedJob)
			failedJobStruct := &ProofJob{
				ID:        job.ID + "_failed",
				Type:      "failed",
				Payload:   json.RawMessage(failedData),
				CreatedAt: time.Now(),
			}

			err = w.queue.EnqueueProof("zk_failed_queue", failedJobStruct)
			if err != nil {
				logging.Logger().Error().Err(err).Str("job_id", job.ID).Msg("Failed to enqueue cached failure")
			}
			w.queue.StoreInputHash(job.ID, inputHash)
			return
		}
	}

	// No cached result found, proceed with normal processing
	// Store the input hash for this job to enable future deduplication
	w.queue.StoreInputHash(job.ID, inputHash)

	w.semaphore <- struct{}{}

	go func(job *ProofJob) {
		defer func() {
			<-w.semaphore
		}()

		proofStartTime := time.Now()

		logging.Logger().Info().
			Str("job_id", job.ID).
			Str("queue", w.queueName).
			Msg("Starting proof generation")

		processingJob := &ProofJob{
			ID:        job.ID + "_processing",
			Type:      "processing",
			Payload:   job.Payload,
			CreatedAt: time.Now(),
		}
		err := w.queue.EnqueueProof(w.processingQueueName, processingJob)
		if err != nil {
			logging.Logger().Error().
				Err(err).
				Str("job_id", job.ID).
				Str("processing_queue", w.processingQueueName).
				Msg("Failed to add job to processing queue")
			return
		}

		proof, err := w.generateProof(job)
		w.removeFromProcessingQueue(job.ID)

		proofDuration := time.Since(proofStartTime)

		if err != nil {
			logging.Logger().Error().
				Err(err).
				Str("job_id", job.ID).
				Str("queue", w.queueName).
				Dur("duration", proofDuration).
				Msg("Failed to process proof job")

			w.addToFailedQueue(job, err)
		} else {
			// Store result with timing information
			proofWithTiming := &common.ProofWithTiming{
				Proof:           proof,
				ProofDurationMs: proofDuration.Milliseconds(),
			}

			resultData, _ := json.Marshal(proofWithTiming)
			resultJob := &ProofJob{
				ID:        job.ID,
				Type:      "result",
				Payload:   json.RawMessage(resultData),
				CreatedAt: time.Now(),
			}
			if enqueueErr := w.queue.EnqueueProof("zk_results_queue", resultJob); enqueueErr != nil {
				logging.Logger().Error().
					Err(enqueueErr).
					Str("job_id", job.ID).
					Msg("Failed to enqueue result")
			}
			if storeErr := w.queue.StoreResult(job.ID, proofWithTiming); storeErr != nil {
				logging.Logger().Error().
					Err(storeErr).
					Str("job_id", job.ID).
					Msg("Failed to store result")
			}

			logging.Logger().Info().
				Str("job_id", job.ID).
				Str("queue", w.queueName).
				Dur("duration", proofDuration).
				Int64("duration_ms", proofDuration.Milliseconds()).
				Msg("Proof job completed successfully")
		}

		// Clean up job metadata now that the job is complete (success or failure)
		if delErr := w.queue.DeleteJobMeta(job.ID); delErr != nil {
			logging.Logger().Warn().
				Err(delErr).
				Str("job_id", job.ID).
				Msg("Failed to delete job metadata (non-critical)")
		}
	}(job)
}

func (w *UpdateQueueWorker) Start() {
	w.BaseQueueWorker.Start()
}

func (w *UpdateQueueWorker) Stop() {
	w.BaseQueueWorker.Stop()
}

func (w *AppendQueueWorker) Start() {
	w.BaseQueueWorker.Start()
}

func (w *AppendQueueWorker) Stop() {
	w.BaseQueueWorker.Stop()
}

func (w *AddressAppendQueueWorker) Start() {
	w.BaseQueueWorker.Start()
}

func (w *AddressAppendQueueWorker) Stop() {
	w.BaseQueueWorker.Stop()
}

// generateProof generates a proof for the given job and returns it.
// Result storage is handled by the caller to include timing information.
func (w *BaseQueueWorker) generateProof(job *ProofJob) (*common.Proof, error) {
	proofRequestMeta, err := common.ParseProofRequestMeta(job.Payload)
	if err != nil {
		return nil, fmt.Errorf("failed to parse proof request: %w", err)
	}

	timer := StartProofTimer(string(proofRequestMeta.CircuitType))
	RecordCircuitInputSize(string(proofRequestMeta.CircuitType), len(job.Payload))

	var proof *common.Proof
	var proofError error

	log.Printf("proofRequestMeta.CircuitType: %s", proofRequestMeta.CircuitType)

	switch proofRequestMeta.CircuitType {
	case common.InclusionCircuitType:
		proof, proofError = w.processInclusionProof(job.Payload, proofRequestMeta)
	case common.NonInclusionCircuitType:
		proof, proofError = w.processNonInclusionProof(job.Payload, proofRequestMeta)
	case common.CombinedCircuitType:
		proof, proofError = w.processCombinedProof(job.Payload, proofRequestMeta)
	case common.BatchUpdateCircuitType:
		proof, proofError = w.processBatchUpdateProof(job.Payload)
	case common.BatchAppendCircuitType:
		proof, proofError = w.processBatchAppendProof(job.Payload)
	case common.BatchAddressAppendCircuitType:
		proof, proofError = w.processBatchAddressAppendProof(job.Payload)
	default:
		return nil, fmt.Errorf("unknown circuit type: %s", proofRequestMeta.CircuitType)
	}

	if proofError != nil {
		timer.ObserveError("proof_generation_failed")
		RecordJobComplete(false)
		return nil, proofError
	}

	timer.ObserveDuration()
	RecordJobComplete(true)

	if proof != nil {
		proofBytes, _ := json.Marshal(proof)
		RecordProofSize(string(proofRequestMeta.CircuitType), len(proofBytes))
	}

	return proof, nil
}

func (w *BaseQueueWorker) processInclusionProof(payload json.RawMessage, meta common.ProofRequestMeta) (*common.Proof, error) {
	ps, err := w.keyManager.GetMerkleSystem(
		meta.StateTreeHeight,
		meta.NumInputs,
		0,
		0,
		meta.Version,
	)
	if err != nil {
		return nil, fmt.Errorf("inclusion proof: %w", err)
	}

	switch meta.Version {
	case 1:
		var params v1.InclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy inclusion parameters: %w", err)
		}
		return v1.ProveInclusion(ps, &params)
	case 2:
		var params v2.InclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal inclusion parameters: %w", err)
		}
		return v2.ProveInclusion(ps, &params)
	}

	return nil, fmt.Errorf("unsupported version: %d", meta.Version)
}

func (w *BaseQueueWorker) processNonInclusionProof(payload json.RawMessage, meta common.ProofRequestMeta) (*common.Proof, error) {
	ps, err := w.keyManager.GetMerkleSystem(
		0,
		0,
		meta.AddressTreeHeight,
		meta.NumAddresses,
		meta.Version,
	)
	if err != nil {
		return nil, fmt.Errorf("non-inclusion proof: %w", err)
	}

	if meta.AddressTreeHeight == 26 {
		var params v1.NonInclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy non-inclusion parameters: %w", err)
		}
		return v1.ProveNonInclusion(ps, &params)
	} else if meta.AddressTreeHeight == 40 {
		var params v2.NonInclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal non-inclusion parameters: %w", err)
		}
		return v2.ProveNonInclusion(ps, &params)
	}

	return nil, fmt.Errorf("unsupported address tree height: %d", meta.AddressTreeHeight)
}

func (w *BaseQueueWorker) processCombinedProof(payload json.RawMessage, meta common.ProofRequestMeta) (*common.Proof, error) {
	ps, err := w.keyManager.GetMerkleSystem(
		meta.StateTreeHeight,
		meta.NumInputs,
		meta.AddressTreeHeight,
		meta.NumAddresses,
		meta.Version,
	)
	if err != nil {
		return nil, fmt.Errorf("combined proof: %w", err)
	}

	switch meta.AddressTreeHeight {
	case 26:
		var params v1.CombinedParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy combined parameters: %w", err)
		}
		return v1.ProveCombined(ps, &params)
	case 40:
		var params v2.CombinedParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal combined parameters: %w", err)
		}
		return v2.ProveCombined(ps, &params)
	}

	return nil, fmt.Errorf("unsupported address tree height: %d", meta.AddressTreeHeight)
}

func (w *BaseQueueWorker) processBatchUpdateProof(payload json.RawMessage) (*common.Proof, error) {
	var params v2.BatchUpdateParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch update parameters: %w", err)
	}

	ps, err := w.keyManager.GetBatchSystem(
		common.BatchUpdateCircuitType,
		params.Height,
		params.BatchSize,
	)
	if err != nil {
		return nil, fmt.Errorf("batch update proof: %w", err)
	}

	return v2.ProveBatchUpdate(ps, &params)
}

func (w *BaseQueueWorker) processBatchAppendProof(payload json.RawMessage) (*common.Proof, error) {
	var params v2.BatchAppendParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch append parameters: %w", err)
	}

	ps, err := w.keyManager.GetBatchSystem(
		common.BatchAppendCircuitType,
		params.Height,
		params.BatchSize,
	)
	if err != nil {
		return nil, fmt.Errorf("batch append proof: %w", err)
	}

	return v2.ProveBatchAppend(ps, &params)
}

func (w *BaseQueueWorker) processBatchAddressAppendProof(payload json.RawMessage) (*common.Proof, error) {
	var params v2.BatchAddressAppendParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch address append parameters: %w", err)
	}

	ps, err := w.keyManager.GetBatchSystem(
		common.BatchAddressAppendCircuitType,
		params.TreeHeight,
		params.BatchSize,
	)
	if err != nil {
		return nil, fmt.Errorf("batch address append proof: %w", err)
	}

	logging.Logger().Info().Msg("Processing batch address append proof")
	return v2.ProveBatchAddressAppend(ps, &params)
}

func (w *BaseQueueWorker) removeFromProcessingQueue(jobID string) {
	processingQueueLength, _ := w.queue.Client.LLen(w.queue.Ctx, w.processingQueueName).Result()

	for i := range processingQueueLength {
		item, err := w.queue.Client.LIndex(w.queue.Ctx, w.processingQueueName, i).Result()
		if err != nil {
			continue
		}

		var job ProofJob
		if json.Unmarshal([]byte(item), &job) == nil && job.ID == jobID+"_processing" {
			w.queue.Client.LRem(w.queue.Ctx, w.processingQueueName, 1, item)
			break
		}
	}
}

func (w *BaseQueueWorker) addToFailedQueue(job *ProofJob, err error) {
	failedJob := map[string]interface{}{
		"original_job": job,
		"error":        err.Error(),
		"failed_at":    time.Now(),
	}

	failedData, _ := json.Marshal(failedJob)
	failedJobStruct := &ProofJob{
		ID:        job.ID + "_failed",
		Type:      "failed",
		Payload:   json.RawMessage(failedData),
		CreatedAt: time.Now(),
	}

	err = w.queue.EnqueueProof("zk_failed_queue", failedJobStruct)
	if err != nil {
		return
	}
}
