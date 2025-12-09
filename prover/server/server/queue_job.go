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
	"runtime"
	"strconv"
	"time"
)

const (
	// JobExpirationTimeout should match the forester's max_wait_time (600 seconds)
	JobExpirationTimeout = 600 * time.Second

	// Memory estimates per circuit type (in GB)
	// Based on live measurements: ~11GB per batch-500 proof
	// batch_update_32_500:        ~11GB (8M constraints)
	// batch_append_32_500:        ~11GB (7.8M constraints)
	// batch_address-append_40_250: ~15GB (larger tree height)
	//
	// For safety, we use the largest (address-append) as the baseline
	MemoryPerProofGB = 15

	// MemoryReserveGB is memory to reserve for OS, proving keys, and other processes
	// Proving keys can be 10-20GB depending on which circuits are loaded
	MemoryReserveGB = 20

	// NumQueueWorkers is the number of queue workers (update, append, address-append)
	NumQueueWorkers = 3

	// MinConcurrencyPerWorker is the minimum concurrency per worker
	MinConcurrencyPerWorker = 1

	// MaxConcurrencyPerWorker is the maximum concurrency per worker (safety cap)
	MaxConcurrencyPerWorker = 100
)

// getMaxConcurrency calculates optimal concurrency based on available system memory.
// Falls back to PROVER_MAX_CONCURRENCY env var if set, otherwise auto-calculates.
func getMaxConcurrency() int {
	// First check for explicit override
	if val := os.Getenv("PROVER_MAX_CONCURRENCY"); val != "" {
		if concurrency, err := strconv.Atoi(val); err == nil && concurrency > 0 {
			logging.Logger().Info().
				Int("max_concurrency", concurrency).
				Msg("Using custom max concurrency from PROVER_MAX_CONCURRENCY")
			return concurrency
		}
	}

	// Auto-calculate based on system memory
	concurrency := calculateConcurrencyFromMemory()

	logging.Logger().Info().
		Int("max_concurrency", concurrency).
		Msg("Using auto-calculated max concurrency based on system memory")

	return concurrency
}

// calculateConcurrencyFromMemory determines optimal per-worker concurrency based on RAM.
// Formula: (TotalRAM - Reserve) / (MemoryPerProof * NumWorkers)
func calculateConcurrencyFromMemory() int {
	var memStats runtime.MemStats
	runtime.ReadMemStats(&memStats)

	// Get total system memory (Sys includes all memory obtained from OS)
	// Note: This is Go's view of memory, for more accurate system total we read from OS
	totalMemGB := getTotalSystemMemoryGB()

	availableMemGB := totalMemGB - MemoryReserveGB
	if availableMemGB < MemoryPerProofGB {
		logging.Logger().Warn().
			Int("total_mem_gb", totalMemGB).
			Int("reserve_gb", MemoryReserveGB).
			Int("available_gb", availableMemGB).
			Msg("Very low memory available, using minimum concurrency")
		return MinConcurrencyPerWorker
	}

	// Total concurrent proofs across all workers
	totalConcurrentProofs := availableMemGB / MemoryPerProofGB

	// Divide by number of workers to get per-worker concurrency
	perWorkerConcurrency := totalConcurrentProofs / NumQueueWorkers

	// Apply bounds
	if perWorkerConcurrency < MinConcurrencyPerWorker {
		perWorkerConcurrency = MinConcurrencyPerWorker
	}
	if perWorkerConcurrency > MaxConcurrencyPerWorker {
		perWorkerConcurrency = MaxConcurrencyPerWorker
	}

	logging.Logger().Info().
		Int("total_system_mem_gb", totalMemGB).
		Int("reserve_gb", MemoryReserveGB).
		Int("available_gb", availableMemGB).
		Int("mem_per_proof_gb", MemoryPerProofGB).
		Int("num_workers", NumQueueWorkers).
		Int("total_concurrent_proofs", totalConcurrentProofs).
		Int("per_worker_concurrency", perWorkerConcurrency).
		Msg("Calculated concurrency from system memory")

	return perWorkerConcurrency
}

// getTotalSystemMemoryGB returns total system memory in GB.
// Uses OS-specific methods to get accurate total RAM.
func getTotalSystemMemoryGB() int {
	// Try to read from cgroup (for containerized environments like k8s)
	if memLimit := readCgroupMemoryLimit(); memLimit > 0 {
		memGB := int(memLimit / (1024 * 1024 * 1024))
		logging.Logger().Debug().
			Int64("cgroup_limit_bytes", memLimit).
			Int("cgroup_limit_gb", memGB).
			Msg("Using cgroup memory limit")
		return memGB
	}

	// Fall back to reading from /proc/meminfo (Linux)
	if memTotal := readProcMeminfo(); memTotal > 0 {
		memGB := int(memTotal / (1024 * 1024 * 1024))
		logging.Logger().Debug().
			Int64("proc_meminfo_bytes", memTotal).
			Int("proc_meminfo_gb", memGB).
			Msg("Using /proc/meminfo total")
		return memGB
	}

	// Last resort: use Go's runtime stats (less accurate)
	var memStats runtime.MemStats
	runtime.ReadMemStats(&memStats)
	memGB := int(memStats.Sys / (1024 * 1024 * 1024))
	if memGB < 1 {
		memGB = 8 // Assume at least 8GB if we can't detect
	}

	logging.Logger().Debug().
		Uint64("runtime_sys_bytes", memStats.Sys).
		Int("estimated_gb", memGB).
		Msg("Using Go runtime memory estimate")

	return memGB
}

// readCgroupMemoryLimit reads memory limit from cgroup v2 or v1
func readCgroupMemoryLimit() int64 {
	// Try cgroup v2 first
	if data, err := os.ReadFile("/sys/fs/cgroup/memory.max"); err == nil {
		s := string(data)
		s = s[:len(s)-1] // trim newline
		if s != "max" {
			if limit, err := strconv.ParseInt(s, 10, 64); err == nil {
				return limit
			}
		}
	}

	// Try cgroup v1
	if data, err := os.ReadFile("/sys/fs/cgroup/memory/memory.limit_in_bytes"); err == nil {
		s := string(data)
		s = s[:len(s)-1] // trim newline
		if limit, err := strconv.ParseInt(s, 10, 64); err == nil {
			// Ignore very large values (effectively unlimited)
			if limit < 1<<62 {
				return limit
			}
		}
	}

	return 0
}

// readProcMeminfo reads total memory from /proc/meminfo
func readProcMeminfo() int64 {
	data, err := os.ReadFile("/proc/meminfo")
	if err != nil {
		return 0
	}

	lines := string(data)
	for _, line := range splitLines(lines) {
		if len(line) > 9 && line[:9] == "MemTotal:" {
			// Format: "MemTotal:       16384000 kB"
			fields := splitFields(line)
			if len(fields) >= 2 {
				if kb, err := strconv.ParseInt(fields[1], 10, 64); err == nil {
					return kb * 1024 // Convert KB to bytes
				}
			}
		}
	}

	return 0
}

// splitLines splits string by newlines without importing strings package
func splitLines(s string) []string {
	var lines []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		lines = append(lines, s[start:])
	}
	return lines
}

// splitFields splits string by whitespace without importing strings package
func splitFields(s string) []string {
	var fields []string
	start := -1
	for i := 0; i < len(s); i++ {
		isSpace := s[i] == ' ' || s[i] == '\t'
		if !isSpace && start == -1 {
			start = i
		} else if isSpace && start != -1 {
			fields = append(fields, s[start:i])
			start = -1
		}
	}
	if start != -1 {
		fields = append(fields, s[start:])
	}
	return fields
}

type ProofJob struct {
	ID        string          `json:"id"`
	Type      string          `json:"type"`
	Payload   json.RawMessage `json:"payload"`
	CreatedAt time.Time       `json:"created_at"`
	// TreeID is the merkle tree pubkey - used for fair queuing across trees
	// If empty, job goes to the default queue (backwards compatible)
	TreeID string `json:"tree_id,omitempty"`
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

			// Add to failed queue with new job ID (without full payload to save memory)
			failedJob := map[string]interface{}{
				"original_job": map[string]interface{}{
					"id":           job.ID,
					"type":         job.Type,
					"payload_size": len(job.Payload),
					"created_at":   job.CreatedAt,
				},
				"error":       errorMsg,
				"failed_at":   time.Now(),
				"cached_from": cachedFailedJobID,
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

	go func(job *ProofJob, inputHash string) {
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

			// On failure: clean up in-flight marker to allow retry with new job
			if delErr := w.queue.DeleteInFlightJob(inputHash, job.ID); delErr != nil {
				logging.Logger().Warn().
					Err(delErr).
					Str("job_id", job.ID).
					Str("input_hash", inputHash).
					Msg("Failed to delete in-flight job marker (non-critical)")
			}
			// Clean up job metadata
			if delErr := w.queue.DeleteJobMeta(job.ID); delErr != nil {
				logging.Logger().Warn().
					Err(delErr).
					Str("job_id", job.ID).
					Msg("Failed to delete job metadata (non-critical)")
			}
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

			// On success: DON'T delete in-flight marker - let it expire with the result.
			// This allows future requests with identical inputs to get the cached result
			// instead of creating a new job. Both marker and result have 10-min TTL.
			// Only clean up job metadata (no longer needed since result is stored).
			if delErr := w.queue.DeleteJobMeta(job.ID); delErr != nil {
				logging.Logger().Warn().
					Err(delErr).
					Str("job_id", job.ID).
					Msg("Failed to delete job metadata (non-critical)")
			}
		}
	}(job, inputHash)
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
	// Extract circuit type from payload for debugging, but don't store full payload
	// to prevent memory issues (payloads can be hundreds of KB)
	var circuitType string
	var payloadMeta map[string]interface{}
	if json.Unmarshal(job.Payload, &payloadMeta) == nil {
		if ct, ok := payloadMeta["circuitType"].(string); ok {
			circuitType = ct
		}
	}

	failedJob := map[string]interface{}{
		"original_job": map[string]interface{}{
			"id":           job.ID,
			"type":         job.Type,
			"circuit_type": circuitType,
			"payload_size": len(job.Payload),
			"created_at":   job.CreatedAt,
		},
		"error":     err.Error(),
		"failed_at": time.Now(),
	}

	failedData, _ := json.Marshal(failedJob)
	failedJobStruct := &ProofJob{
		ID:        job.ID + "_failed",
		Type:      "failed",
		Payload:   json.RawMessage(failedData),
		CreatedAt: time.Now(),
	}

	enqueueErr := w.queue.EnqueueProof("zk_failed_queue", failedJobStruct)
	if enqueueErr != nil {
		logging.Logger().Error().
			Err(enqueueErr).
			Str("job_id", job.ID).
			Msg("Failed to add job to failed queue")
	}
}
