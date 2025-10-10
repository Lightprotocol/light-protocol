package server

import (
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"light/light-prover/prover/v1"
	"light/light-prover/prover/v2"
	"log"
	"time"
)

const (
	// JobExpirationTimeout should match the forester's max_wait_time (600 seconds)
	JobExpirationTimeout = 600 * time.Second
)

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
	return &UpdateQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_update_queue",
			processingQueueName: "zk_update_processing_queue",
		},
	}
}

func NewAppendQueueWorker(redisQueue *RedisQueue, keyManager *common.LazyKeyManager) *AppendQueueWorker {
	return &AppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_append_queue",
			processingQueueName: "zk_append_processing_queue",
		},
	}
}

func NewAddressAppendQueueWorker(redisQueue *RedisQueue, keyManager *common.LazyKeyManager) *AddressAppendQueueWorker {
	return &AddressAppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			keyManager:          keyManager,
			stopChan:            make(chan struct{}),
			queueName:           "zk_address_append_queue",
			processingQueueName: "zk_address_append_processing_queue",
		},
	}
}

func (w *BaseQueueWorker) Start() {
	logging.Logger().Info().Str("queue", w.queueName).Msg("Starting queue worker")

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
		if w.queueName == "zk_update_queue" {
			circuitType = "update"
		} else if w.queueName == "zk_append_queue" {
			circuitType = "append"
		} else if w.queueName == "zk_address_append_queue" {
			circuitType = "address-append"
		}
		QueueWaitTime.WithLabelValues(circuitType).Observe(queueWaitTime)
	}

	logging.Logger().Info().
		Str("job_id", job.ID).
		Str("job_type", job.Type).
		Str("queue", w.queueName).
		Msg("Processing proof job")

	processingJob := &ProofJob{
		ID:        job.ID + "_processing",
		Type:      "processing",
		Payload:   job.Payload,
		CreatedAt: time.Now(),
	}
	err = w.queue.EnqueueProof(w.processingQueueName, processingJob)
	if err != nil {
		return
	}

	err = w.processProofJob(job)
	w.removeFromProcessingQueue(job.ID)

	if err != nil {
		logging.Logger().Error().
			Err(err).
			Str("job_id", job.ID).
			Str("queue", w.queueName).
			Msg("Failed to process proof job")

		w.addToFailedQueue(job, err)
	}
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

func (w *BaseQueueWorker) processProofJob(job *ProofJob) error {
	proofRequestMeta, err := common.ParseProofRequestMeta(job.Payload)
	if err != nil {
		return fmt.Errorf("failed to parse proof request: %w", err)
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
		return fmt.Errorf("unknown circuit type: %s", proofRequestMeta.CircuitType)
	}

	if proofError != nil {
		timer.ObserveError("proof_generation_failed")
		RecordJobComplete(false)
		return proofError
	}

	timer.ObserveDuration()
	RecordJobComplete(true)

	if proof != nil {
		proofBytes, _ := json.Marshal(proof)
		RecordProofSize(string(proofRequestMeta.CircuitType), len(proofBytes))
	}

	resultData, _ := json.Marshal(proof)
	resultJob := &ProofJob{
		ID:        job.ID,
		Type:      "result",
		Payload:   json.RawMessage(resultData),
		CreatedAt: time.Now(),
	}
	err = w.queue.EnqueueProof("zk_results_queue", resultJob)
	if err != nil {
		return err
	}
	return w.queue.StoreResult(job.ID, proof)
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

	if meta.Version == 1 {
		var params v1.InclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy inclusion parameters: %w", err)
		}
		return v1.ProveInclusion(ps, &params)
	} else if meta.Version == 2 {
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

	if meta.AddressTreeHeight == 26 {
		var params v1.CombinedParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy combined parameters: %w", err)
		}
		return v1.ProveCombined(ps, &params)
	} else if meta.AddressTreeHeight == 40 {
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

	for i := int64(0); i < processingQueueLength; i++ {
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
