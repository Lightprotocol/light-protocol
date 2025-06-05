package server

import (
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"time"
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
	provingSystemsV1    []*prover.ProvingSystemV1
	provingSystemsV2    []*prover.ProvingSystemV2
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

func NewUpdateQueueWorker(redisQueue *RedisQueue, psv1 []*prover.ProvingSystemV1, psv2 []*prover.ProvingSystemV2) *UpdateQueueWorker {
	return &UpdateQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			provingSystemsV1:    psv1,
			provingSystemsV2:    psv2,
			stopChan:            make(chan struct{}),
			queueName:           "zk_update_queue",
			processingQueueName: "zk_update_processing_queue",
		},
	}
}

func NewAppendQueueWorker(redisQueue *RedisQueue, psv1 []*prover.ProvingSystemV1, psv2 []*prover.ProvingSystemV2) *AppendQueueWorker {
	return &AppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			provingSystemsV1:    psv1,
			provingSystemsV2:    psv2,
			stopChan:            make(chan struct{}),
			queueName:           "zk_append_queue",
			processingQueueName: "zk_append_processing_queue",
		},
	}
}

func NewAddressAppendQueueWorker(redisQueue *RedisQueue, psv1 []*prover.ProvingSystemV1, psv2 []*prover.ProvingSystemV2) *AddressAppendQueueWorker {
	return &AddressAppendQueueWorker{
		BaseQueueWorker: &BaseQueueWorker{
			queue:               redisQueue,
			provingSystemsV1:    psv1,
			provingSystemsV2:    psv2,
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
	w.queue.EnqueueProof(w.processingQueueName, processingJob)

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
	proofRequestMeta, err := prover.ParseProofRequestMeta(job.Payload)
	if err != nil {
		return fmt.Errorf("failed to parse proof request: %w", err)
	}

	var proof *prover.Proof
	var proofError error

	switch proofRequestMeta.CircuitType {
	case prover.InclusionCircuitType:
		proof, proofError = w.processInclusionProof(job.Payload, proofRequestMeta)
	case prover.NonInclusionCircuitType:
		proof, proofError = w.processNonInclusionProof(job.Payload, proofRequestMeta)
	case prover.CombinedCircuitType:
		proof, proofError = w.processCombinedProof(job.Payload, proofRequestMeta)
	case prover.BatchUpdateCircuitType:
		proof, proofError = w.processBatchUpdateProof(job.Payload)
	case prover.BatchAppendWithProofsCircuitType:
		proof, proofError = w.processBatchAppendWithProofsProof(job.Payload)
	case prover.BatchAddressAppendCircuitType:
		proof, proofError = w.processBatchAddressAppendProof(job.Payload)
	default:
		return fmt.Errorf("unknown circuit type: %s", proofRequestMeta.CircuitType)
	}

	if proofError != nil {
		return proofError
	}

	resultData, _ := json.Marshal(proof)
	resultJob := &ProofJob{
		ID:        job.ID,
		Type:      "result",
		Payload:   json.RawMessage(resultData),
		CreatedAt: time.Now(),
	}
	w.queue.EnqueueProof("zk_results_queue", resultJob)
	return w.queue.StoreResult(job.ID, proof)
}

func (w *BaseQueueWorker) processInclusionProof(payload json.RawMessage, meta prover.ProofRequestMeta) (*prover.Proof, error) {
	var ps *prover.ProvingSystemV1
	for _, provingSystem := range w.provingSystemsV1 {
		if provingSystem.InclusionNumberOfCompressedAccounts == uint32(meta.NumInputs) &&
			provingSystem.InclusionTreeHeight == uint32(meta.StateTreeHeight) &&
			provingSystem.Version == uint32(meta.Version) &&
			provingSystem.NonInclusionNumberOfCompressedAccounts == uint32(0) {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, fmt.Errorf("no proving system found for inclusion proof with meta: %+v", meta)
	}

	if meta.Version == 0 {
		var params prover.LegacyInclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy inclusion parameters: %w", err)
		}
		return ps.LegacyProveInclusion(&params)
	} else if meta.Version == 1 {
		var params prover.InclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal inclusion parameters: %w", err)
		}
		return ps.ProveInclusion(&params)
	}

	return nil, fmt.Errorf("unsupported version: %d", meta.Version)
}

func (w *BaseQueueWorker) processNonInclusionProof(payload json.RawMessage, meta prover.ProofRequestMeta) (*prover.Proof, error) {
	var ps *prover.ProvingSystemV1
	for _, provingSystem := range w.provingSystemsV1 {
		if provingSystem.NonInclusionNumberOfCompressedAccounts == uint32(meta.NumAddresses) &&
			provingSystem.NonInclusionTreeHeight == uint32(meta.AddressTreeHeight) &&
			provingSystem.InclusionNumberOfCompressedAccounts == uint32(0) {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, fmt.Errorf("no proving system found for non-inclusion proof with meta: %+v", meta)
	}

	if meta.AddressTreeHeight == 26 {
		var params prover.LegacyNonInclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy non-inclusion parameters: %w", err)
		}
		return ps.LegacyProveNonInclusion(&params)
	} else if meta.AddressTreeHeight == 40 {
		var params prover.NonInclusionParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal non-inclusion parameters: %w", err)
		}
		return ps.ProveNonInclusion(&params)
	}

	return nil, fmt.Errorf("unsupported address tree height: %d", meta.AddressTreeHeight)
}

func (w *BaseQueueWorker) processCombinedProof(payload json.RawMessage, meta prover.ProofRequestMeta) (*prover.Proof, error) {
	var ps *prover.ProvingSystemV1
	for _, provingSystem := range w.provingSystemsV1 {
		if provingSystem.InclusionNumberOfCompressedAccounts == meta.NumInputs &&
			provingSystem.NonInclusionNumberOfCompressedAccounts == meta.NumAddresses &&
			provingSystem.InclusionTreeHeight == meta.StateTreeHeight &&
			provingSystem.NonInclusionTreeHeight == meta.AddressTreeHeight {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, fmt.Errorf("no proving system found for combined proof with meta: %+v", meta)
	}

	if meta.AddressTreeHeight == 26 {
		var params prover.LegacyCombinedParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal legacy combined parameters: %w", err)
		}
		return ps.LegacyProveCombined(&params)
	} else if meta.AddressTreeHeight == 40 {
		var params prover.CombinedParameters
		if err := json.Unmarshal(payload, &params); err != nil {
			return nil, fmt.Errorf("failed to unmarshal combined parameters: %w", err)
		}
		return ps.ProveCombined(&params)
	}

	return nil, fmt.Errorf("unsupported address tree height: %d", meta.AddressTreeHeight)
}

func (w *BaseQueueWorker) processBatchUpdateProof(payload json.RawMessage) (*prover.Proof, error) {
	var params prover.BatchUpdateParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch update parameters: %w", err)
	}

	for _, provingSystem := range w.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchUpdateCircuitType &&
			provingSystem.TreeHeight == params.Height &&
			provingSystem.BatchSize == params.BatchSize {
			return provingSystem.ProveBatchUpdate(&params)
		}
	}

	return nil, fmt.Errorf("no proving system found for batch update with height %d and batch size %d", params.Height, params.BatchSize)
}

func (w *BaseQueueWorker) processBatchAppendWithProofsProof(payload json.RawMessage) (*prover.Proof, error) {
	var params prover.BatchAppendWithProofsParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch append parameters: %w", err)
	}

	for _, provingSystem := range w.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchAppendWithProofsCircuitType &&
			provingSystem.TreeHeight == params.Height &&
			provingSystem.BatchSize == params.BatchSize {
			return provingSystem.ProveBatchAppendWithProofs(&params)
		}
	}

	return nil, fmt.Errorf("no proving system found for batch append with height %d and batch size %d", params.Height, params.BatchSize)
}

func (w *BaseQueueWorker) processBatchAddressAppendProof(payload json.RawMessage) (*prover.Proof, error) {
	var params prover.BatchAddressAppendParameters
	if err := json.Unmarshal(payload, &params); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch address append parameters: %w", err)
	}

	for _, provingSystem := range w.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchAddressAppendCircuitType &&
			provingSystem.TreeHeight == params.TreeHeight &&
			provingSystem.BatchSize == params.BatchSize {
			return provingSystem.ProveBatchAddressAppend(&params)
		}
	}

	return nil, fmt.Errorf("no proving system found for batch address append with height %d and batch size %d", params.TreeHeight, params.BatchSize)
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

	w.queue.EnqueueProof("zk_failed_queue", failedJobStruct)
}
