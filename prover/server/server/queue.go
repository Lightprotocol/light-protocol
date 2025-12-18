package server

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"time"

	"github.com/google/uuid"
	"github.com/redis/go-redis/v9"
)

const (
	// ResultsIndexKey is the Redis hash that maps inputHash → jobID
	ResultsIndexKey = "zk_results_index"
	// FailedIndexKey is the Redis hash that maps inputHash → jobID
	FailedIndexKey = "zk_failed_index"
)

type RedisQueue struct {
	Client *redis.Client
	Ctx    context.Context
}

func NewRedisQueue(redisURL string) (*RedisQueue, error) {
	opts, err := redis.ParseURL(redisURL)
	if err != nil {
		return nil, fmt.Errorf("failed to parse Redis URL: %w", err)
	}

	// Configure connection pool and timeouts for Cloud Run + VPC connector reliability
	opts.PoolSize = 500                    // Connection pool size per instance (increased for high load)
	opts.MinIdleConns = 10                 // Keep some connections warm
	opts.DialTimeout = 10 * time.Second    // Timeout for establishing new connections
	opts.ReadTimeout = 30 * time.Second    // Timeout for read operations (BLPOP can be slow)
	opts.WriteTimeout = 10 * time.Second   // Timeout for write operations
	opts.PoolTimeout = 15 * time.Second    // Timeout for getting connection from pool
	opts.ConnMaxIdleTime = 5 * time.Minute // Close idle connections after this time
	opts.MaxRetries = 3                    // Retry failed commands

	client := redis.NewClient(opts)
	ctx := context.Background()

	ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()

	if err := client.Ping(ctx).Err(); err != nil {
		return nil, fmt.Errorf("failed to connect to Redis: %w", err)
	}

	logging.Logger().Info().
		Int("pool_size", opts.PoolSize).
		Int("min_idle_conns", opts.MinIdleConns).
		Dur("dial_timeout", opts.DialTimeout).
		Dur("read_timeout", opts.ReadTimeout).
		Dur("write_timeout", opts.WriteTimeout).
		Int("max_retries", opts.MaxRetries).
		Msg("Redis client configured with connection pool")

	return &RedisQueue{Client: client, Ctx: context.Background()}, nil
}

func (rq *RedisQueue) EnqueueProof(queueName string, job *ProofJob) error {
	data, err := json.Marshal(job)
	if err != nil {
		return fmt.Errorf("failed to marshal job: %w", err)
	}

	// Use tree-specific sub-queue for fair queuing if TreeID is set
	actualQueueName := queueName
	if job.TreeID != "" && isFairQueueEnabled(queueName) {
		actualQueueName = fmt.Sprintf("%s:%s", queueName, job.TreeID)
		// Track this tree in the trees set for round-robin
		treesSetKey := fmt.Sprintf("%s:trees", queueName)
		rq.Client.SAdd(rq.Ctx, treesSetKey, job.TreeID)
	}

	err = rq.Client.RPush(rq.Ctx, actualQueueName, data).Err()
	if err != nil {
		return fmt.Errorf("failed to enqueue job: %w", err)
	}

	logging.Logger().Info().
		Str("job_id", job.ID).
		Str("queue", actualQueueName).
		Str("tree_id", job.TreeID).
		Str("redis_addr", rq.Client.Options().Addr).
		Msg("Job enqueued successfully")
	return nil
}

// isFairQueueEnabled returns true for queues that support fair queuing per tree
func isFairQueueEnabled(queueName string) bool {
	return queueName == "zk_update_queue" ||
		queueName == "zk_append_queue" ||
		queueName == "zk_address_append_queue"
}

// StoreJobMeta stores job metadata when a job is submitted to enable reliable status lookups.
// This ensures the status endpoint can find the job even before a worker picks it up.
// TTL is set to 1 hour to match result TTL.
func (rq *RedisQueue) StoreJobMeta(jobID string, queueName string, circuitType string) error {
	key := fmt.Sprintf("zk_job_meta_%s", jobID)
	meta := map[string]interface{}{
		"queue":        queueName,
		"circuit_type": circuitType,
		"submitted_at": time.Now(),
		"status":       "queued",
	}
	data, err := json.Marshal(meta)
	if err != nil {
		return fmt.Errorf("failed to marshal job meta: %w", err)
	}

	err = rq.Client.Set(rq.Ctx, key, data, 1*time.Hour).Err()
	if err != nil {
		return fmt.Errorf("failed to store job meta: %w", err)
	}

	logging.Logger().Debug().
		Str("job_id", jobID).
		Str("queue", queueName).
		Str("circuit_type", circuitType).
		Str("redis_addr", rq.Client.Options().Addr).
		Msg("Stored job metadata for status tracking")

	return nil
}

// GetJobMeta retrieves job metadata by job ID.
// Returns nil if the job metadata doesn't exist.
func (rq *RedisQueue) GetJobMeta(jobID string) (map[string]interface{}, error) {
	key := fmt.Sprintf("zk_job_meta_%s", jobID)
	result, err := rq.Client.Get(rq.Ctx, key).Result()
	if err == redis.Nil {
		return nil, nil
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get job meta: %w", err)
	}

	var meta map[string]interface{}
	if err := json.Unmarshal([]byte(result), &meta); err != nil {
		return nil, fmt.Errorf("failed to unmarshal job meta: %w", err)
	}

	return meta, nil
}

// DeleteJobMeta removes job metadata when a job completes or fails.
func (rq *RedisQueue) DeleteJobMeta(jobID string) error {
	key := fmt.Sprintf("zk_job_meta_%s", jobID)
	return rq.Client.Del(rq.Ctx, key).Err()
}

func (rq *RedisQueue) DequeueProof(queueName string, timeout time.Duration) (*ProofJob, error) {
	// Check if this queue supports fair queuing
	if isFairQueueEnabled(queueName) {
		return rq.dequeueWithFairQueuing(queueName, timeout)
	}

	// Standard dequeue for non-fair queues
	result, err := rq.Client.BLPop(rq.Ctx, timeout, queueName).Result()
	if err != nil {
		if err == redis.Nil {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to dequeue job: %w", err)
	}

	if len(result) < 2 {
		return nil, fmt.Errorf("invalid result from Redis")
	}

	var job ProofJob
	err = json.Unmarshal([]byte(result[1]), &job)
	if err != nil {
		return nil, fmt.Errorf("failed to unmarshal job: %w", err)
	}

	return &job, nil
}

// dequeueWithFairQueuing implements round-robin dequeuing across tree-specific sub-queues
// Within each tree's queue, it prioritizes jobs with lower batch_index to ensure sequential processing
func (rq *RedisQueue) dequeueWithFairQueuing(queueName string, timeout time.Duration) (*ProofJob, error) {
	treesSetKey := fmt.Sprintf("%s:trees", queueName)
	lastTreeKey := fmt.Sprintf("%s:last_tree", queueName)

	// Get all trees with pending jobs
	trees, err := rq.Client.SMembers(rq.Ctx, treesSetKey).Result()
	if err != nil {
		return nil, fmt.Errorf("failed to get trees set: %w", err)
	}

	// If no trees with jobs, fall back to main queue (for jobs without tree_id)
	if len(trees) == 0 {
		result, err := rq.Client.BLPop(rq.Ctx, timeout, queueName).Result()
		if err != nil {
			if err == redis.Nil {
				return nil, nil
			}
			return nil, fmt.Errorf("failed to dequeue job: %w", err)
		}
		if len(result) < 2 {
			return nil, fmt.Errorf("invalid result from Redis")
		}
		var job ProofJob
		err = json.Unmarshal([]byte(result[1]), &job)
		if err != nil {
			return nil, fmt.Errorf("failed to unmarshal job: %w", err)
		}
		return &job, nil
	}

	// Get the last processed tree to start round-robin from next
	lastTree, _ := rq.Client.Get(rq.Ctx, lastTreeKey).Result()

	// Find starting index for round-robin
	startIdx := 0
	for i, tree := range trees {
		if tree == lastTree {
			startIdx = (i + 1) % len(trees)
			break
		}
	}

	// Try each tree in round-robin order
	for i := range len(trees) {
		idx := (startIdx + i) % len(trees)
		tree := trees[idx]
		subQueueName := fmt.Sprintf("%s:%s", queueName, tree)

		// Get job with lowest batch_index from this tree's queue
		job, err := rq.dequeueLowestBatchIndex(subQueueName)
		if err == redis.Nil || job == nil {
			// Queue empty, remove tree from set
			rq.Client.SRem(rq.Ctx, treesSetKey, tree)
			continue
		}
		if err != nil {
			logging.Logger().Warn().
				Err(err).
				Str("queue", subQueueName).
				Msg("Error getting lowest batch_index job from tree sub-queue")
			continue
		}

		// Update last processed tree for next round-robin
		rq.Client.Set(rq.Ctx, lastTreeKey, tree, 1*time.Hour)

		// Check if queue is now empty and remove from trees set
		queueLen, _ := rq.Client.LLen(rq.Ctx, subQueueName).Result()
		if queueLen == 0 {
			rq.Client.SRem(rq.Ctx, treesSetKey, tree)
		}

		logging.Logger().Debug().
			Str("job_id", job.ID).
			Str("tree_id", tree).
			Int64("batch_index", job.BatchIndex).
			Str("queue", subQueueName).
			Int("trees_count", len(trees)).
			Msg("Dequeued job with fair queuing and batch_index priority")

		return job, nil
	}

	// All tree queues were empty, try main queue as fallback
	result, err := rq.Client.BLPop(rq.Ctx, timeout, queueName).Result()
	if err != nil {
		if err == redis.Nil {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to dequeue job: %w", err)
	}
	if len(result) < 2 {
		return nil, fmt.Errorf("invalid result from Redis")
	}
	var job ProofJob
	err = json.Unmarshal([]byte(result[1]), &job)
	if err != nil {
		return nil, fmt.Errorf("failed to unmarshal job: %w", err)
	}
	return &job, nil
}

// BatchIndexScanLimit is the maximum number of items to scan when looking for the lowest batch_index.
const BatchIndexScanLimit = 100

// dequeueLowestBatchIndex finds and removes the job with the lowest batch_index from the queue.
// This ensures that batches are processed in order within each tree, enabling the forester
// to send transactions sequentially as proofs complete.
// Jobs with batch_index -1 (legacy) are treated as having the highest priority among themselves
// but after jobs with explicit batch indices.
//
// Scans up to BatchIndexScanLimit items for performance. If the item was removed by another
// worker between find and remove, retries automatically.
func (rq *RedisQueue) dequeueLowestBatchIndex(queueName string) (*ProofJob, error) {
	// Scan up to BatchIndexScanLimit items instead of the entire queue
	items, err := rq.Client.LRange(rq.Ctx, queueName, 0, BatchIndexScanLimit-1).Result()
	if err != nil {
		return nil, err
	}

	if len(items) == 0 {
		return nil, redis.Nil
	}

	if len(items) == 1 {
		result, err := rq.Client.LPop(rq.Ctx, queueName).Result()
		if err != nil {
			return nil, err
		}
		var job ProofJob
		if err := json.Unmarshal([]byte(result), &job); err != nil {
			return nil, err
		}
		return &job, nil
	}

	var lowestJob *ProofJob
	lowestIdx := -1
	lowestBatchIndex := int64(^uint64(0) >> 1)

	for i, item := range items {
		var job ProofJob
		if err := json.Unmarshal([]byte(item), &job); err != nil {
			logging.Logger().Warn().
				Err(err).
				Str("queue", queueName).
				Int("index", i).
				Msg("Failed to unmarshal job while searching for lowest batch_index")
			continue
		}

		// Jobs with batch_index >= 0 have priority over legacy jobs (batch_index -1)
		// Among jobs with batch_index >= 0, lower index wins
		// Among legacy jobs, first in queue wins (FIFO)
		if job.BatchIndex >= 0 {
			if lowestJob == nil || lowestJob.BatchIndex < 0 || job.BatchIndex < lowestBatchIndex {
				lowestJob = &job
				lowestIdx = i
				lowestBatchIndex = job.BatchIndex
			}
		} else if lowestJob == nil || (lowestJob.BatchIndex < 0 && lowestIdx > i) {
			// Legacy job, only take if no better candidate or this is earlier in queue
			lowestJob = &job
			lowestIdx = i
			lowestBatchIndex = job.BatchIndex
		}
	}

	if lowestJob == nil {
		return nil, redis.Nil
	}

	// Remove the selected job from the queue
	itemToRemove := items[lowestIdx]
	removed, err := rq.Client.LRem(rq.Ctx, queueName, 1, itemToRemove).Result()
	if err != nil {
		return nil, fmt.Errorf("failed to remove job from queue: %w", err)
	}

	if removed == 0 {
		// Item was already removed by another worker, retry
		logging.Logger().Debug().
			Str("job_id", lowestJob.ID).
			Str("queue", queueName).
			Msg("Job was already removed from queue, retrying")
		return rq.dequeueLowestBatchIndex(queueName)
	}

	logging.Logger().Debug().
		Str("job_id", lowestJob.ID).
		Int64("batch_index", lowestJob.BatchIndex).
		Int("queue_position", lowestIdx).
		Int("scanned", len(items)).
		Str("queue", queueName).
		Msg("Dequeued job with lowest batch_index")

	return lowestJob, nil
}

func (rq *RedisQueue) GetQueueStats() (map[string]int64, error) {
	stats := make(map[string]int64)

	queues := []string{"zk_update_queue", "zk_append_queue", "zk_address_append_queue", "zk_update_processing_queue", "zk_append_processing_queue", "zk_address_append_processing_queue", "zk_failed_queue", "zk_results_queue"}

	for _, queue := range queues {
		length, err := rq.Client.LLen(rq.Ctx, queue).Result()
		if err != nil {
			logging.Logger().Warn().Err(err).Str("queue", queue).Msg("Failed to get queue length")
			length = 0
		}
		stats[queue] = length

		// For fair-queued queues, also count tree sub-queues
		if isFairQueueEnabled(queue) {
			treesSetKey := fmt.Sprintf("%s:trees", queue)
			trees, err := rq.Client.SMembers(rq.Ctx, treesSetKey).Result()
			if err == nil {
				var totalTreeQueueLen int64
				for _, tree := range trees {
					subQueueName := fmt.Sprintf("%s:%s", queue, tree)
					subLen, _ := rq.Client.LLen(rq.Ctx, subQueueName).Result()
					totalTreeQueueLen += subLen
				}
				stats[queue+"_tree_subqueues"] = totalTreeQueueLen
				stats[queue+"_tree_count"] = int64(len(trees))
			}
		}
	}

	return stats, nil
}

func (rq *RedisQueue) GetQueueHealth() (map[string]interface{}, error) {
	stats, err := rq.GetQueueStats()
	if err != nil {
		return nil, err
	}

	health := make(map[string]interface{})
	health["queue_lengths"] = stats
	health["timestamp"] = time.Now().Unix()

	health["total_pending"] = stats["zk_update_queue"] + stats["zk_append_queue"] + stats["zk_address_append_queue"]
	health["total_processing"] = stats["zk_update_processing_queue"] + stats["zk_append_processing_queue"] + stats["zk_address_append_processing_queue"]
	health["total_failed"] = stats["zk_failed_queue"]
	health["total_results"] = stats["zk_results_queue"]

	stuckJobs := rq.countStuckJobs()
	health["stuck_jobs"] = stuckJobs

	healthStatus := "healthy"
	if stuckJobs > 0 {
		healthStatus = "degraded"
	}
	if health["total_failed"].(int64) > 50 {
		healthStatus = "unhealthy"
	}
	health["status"] = healthStatus

	return health, nil
}

func (rq *RedisQueue) countStuckJobs() int64 {
	stuckTimeout := time.Now().Add(-2 * time.Minute)
	processingQueues := []string{
		"zk_update_processing_queue",
		"zk_append_processing_queue",
		"zk_address_append_processing_queue",
	}

	var totalStuck int64

	for _, queueName := range processingQueues {
		items, err := rq.Client.LRange(rq.Ctx, queueName, 0, -1).Result()
		if err != nil {
			continue
		}

		for _, item := range items {
			var job ProofJob
			if json.Unmarshal([]byte(item), &job) == nil {
				if job.CreatedAt.Before(stuckTimeout) {
					totalStuck++
				}
			}
		}
	}

	return totalStuck
}

func (rq *RedisQueue) GetResult(jobID string) (interface{}, error) {
	key := fmt.Sprintf("zk_result_%s", jobID)
	result, err := rq.Client.Get(rq.Ctx, key).Result()
	if err == nil {
		var proofWithTiming common.ProofWithTiming
		err = json.Unmarshal([]byte(result), &proofWithTiming)
		if err != nil {
			logging.Logger().Error().
				Str("job_id", jobID).
				Err(err).
				Str("result", result).
				Msg("Failed to unmarshal result")

			return nil, fmt.Errorf("failed to unmarshal direct result: %w", err)
		}
		return &proofWithTiming, nil
	}

	if err != redis.Nil {
		return nil, err
	}

	return rq.searchResultInQueue(jobID)
}

func (rq *RedisQueue) searchResultInQueue(jobID string) (interface{}, error) {
	items, err := rq.Client.LRange(rq.Ctx, "zk_results_queue", 0, -1).Result()
	if err != nil {
		return nil, fmt.Errorf("failed to search results queue: %w", err)
	}

	for _, item := range items {
		var resultJob ProofJob
		if json.Unmarshal([]byte(item), &resultJob) == nil {
			if resultJob.ID == jobID && resultJob.Type == "result" {
				var proofWithTiming common.ProofWithTiming
				err = json.Unmarshal(resultJob.Payload, &proofWithTiming)
				if err != nil {
					return nil, fmt.Errorf("failed to unmarshal queued result: %w", err)
				}
				rq.StoreResult(jobID, &proofWithTiming)

				return &proofWithTiming, nil
			}
		}
	}

	return nil, redis.Nil
}

func (rq *RedisQueue) StoreResult(jobID string, result interface{}) error {
	resultData, err := json.Marshal(result)
	if err != nil {
		logging.Logger().Error().
			Str("job_id", jobID).
			Err(err).
			Msg("Failed to marshal result")
		return fmt.Errorf("failed to marshal result: %w", err)
	}

	key := fmt.Sprintf("zk_result_%s", jobID)
	err = rq.Client.Set(rq.Ctx, key, resultData, 1*time.Hour).Err()
	if err != nil {
		return fmt.Errorf("failed to store result: %w", err)
	}

	logging.Logger().Info().
		Str("job_id", jobID).
		Str("key", key).
		Msg("Result stored successfully")

	return nil
}

// IndexResultByHash atomically adds inputHash → jobID to the results index hash.
func (rq *RedisQueue) IndexResultByHash(inputHash, jobID string) error {
	err := rq.Client.HSet(rq.Ctx, ResultsIndexKey, inputHash, jobID).Err()
	if err != nil {
		return fmt.Errorf("failed to index result: %w", err)
	}
	logging.Logger().Debug().
		Str("input_hash", inputHash).
		Str("job_id", jobID).
		Msg("Indexed result by input hash")
	return nil
}

// IndexFailureByHash atomically adds inputHash → jobID to the failed index hash.
func (rq *RedisQueue) IndexFailureByHash(inputHash, jobID string) error {
	err := rq.Client.HSet(rq.Ctx, FailedIndexKey, inputHash, jobID).Err()
	if err != nil {
		return fmt.Errorf("failed to index failure: %w", err)
	}
	logging.Logger().Debug().
		Str("input_hash", inputHash).
		Str("job_id", jobID).
		Msg("Indexed failure by input hash")
	return nil
}

// RemoveResultIndex removes inputHash from the results index hash.
// Called during cleanup to keep the index in sync with the queue.
func (rq *RedisQueue) RemoveResultIndex(inputHash string) error {
	return rq.Client.HDel(rq.Ctx, ResultsIndexKey, inputHash).Err()
}

// RemoveFailureIndex removes inputHash from the failed index hash.
// Called during cleanup to keep the index in sync with the queue.
func (rq *RedisQueue) RemoveFailureIndex(inputHash string) error {
	return rq.Client.HDel(rq.Ctx, FailedIndexKey, inputHash).Err()
}

func (rq *RedisQueue) CleanupOldResults() error {
	// Remove successful results older than 1 hour
	cutoffTime := time.Now().Add(-1 * time.Hour)

	removed, err := rq.cleanupOldRequestsFromQueue("zk_results_queue", cutoffTime)
	if err != nil {
		logging.Logger().Error().
			Err(err).
			Msg("Failed to cleanup old results by time")
		return err
	}

	if removed > 0 {
		logging.Logger().Info().
			Int64("removed_results", removed).
			Time("cutoff_time", cutoffTime).
			Msg("Cleaned up old results by time")
	}

	return nil
}

func (rq *RedisQueue) CleanupOldRequests() error {
	cutoffTime := time.Now().Add(-30 * time.Minute)

	// Queues to clean up old requests from
	queuesToClean := []string{
		"zk_update_queue",
		"zk_append_queue",
		"zk_address_append_queue",
	}

	totalRemoved := int64(0)

	for _, queueName := range queuesToClean {
		// Clean main queue
		removed, err := rq.cleanupOldRequestsFromQueue(queueName, cutoffTime)
		if err != nil {
			logging.Logger().Error().
				Err(err).
				Str("queue", queueName).
				Msg("Failed to cleanup old requests from queue")
		} else {
			totalRemoved += removed
		}

		// Clean tree sub-queues for fair-queued queues
		if isFairQueueEnabled(queueName) {
			treesSetKey := fmt.Sprintf("%s:trees", queueName)
			trees, err := rq.Client.SMembers(rq.Ctx, treesSetKey).Result()
			if err == nil {
				for _, tree := range trees {
					subQueueName := fmt.Sprintf("%s:%s", queueName, tree)
					subRemoved, err := rq.cleanupOldRequestsFromQueue(subQueueName, cutoffTime)
					if err != nil {
						logging.Logger().Error().
							Err(err).
							Str("queue", subQueueName).
							Msg("Failed to cleanup old requests from tree sub-queue")
						continue
					}
					totalRemoved += subRemoved

					// If tree queue is now empty, remove from trees set
					queueLen, _ := rq.Client.LLen(rq.Ctx, subQueueName).Result()
					if queueLen == 0 {
						rq.Client.SRem(rq.Ctx, treesSetKey, tree)
					}
				}
			}
		}
	}

	if totalRemoved > 0 {
		logging.Logger().Info().
			Int64("removed_items", totalRemoved).
			Time("cutoff_time", cutoffTime).
			Msg("Cleaned up old proof requests")
	}

	return nil
}

func (rq *RedisQueue) CleanupOldResultKeys() error {
	ctx := context.Background()

	keys, err := rq.Client.Keys(ctx, "zk_result_*").Result()
	if err != nil {
		return fmt.Errorf("failed to get result keys: %w", err)
	}

	var removedCount int64

	for _, key := range keys {
		ttl, err := rq.Client.TTL(ctx, key).Result()
		if err != nil {
			continue
		}

		if ttl == -1 || ttl < -time.Hour {
			err = rq.Client.Del(ctx, key).Err()
			if err != nil {
				logging.Logger().Error().
					Err(err).
					Str("key", key).
					Msg("Failed to delete old result key")
			} else {
				removedCount++
				logging.Logger().Debug().
					Str("key", key).
					Dur("ttl", ttl).
					Msg("Removed old result key")
			}
		}
	}

	if removedCount > 0 {
		logging.Logger().Info().
			Int64("removed_keys", removedCount).
			Msg("Cleaned up old result keys")
	}

	return nil
}

func (rq *RedisQueue) CleanupStuckProcessingJobs() error {
	// Jobs stuck in processing for more than 10 minutes are considered stuck
	// (proof generation can take 3-4 minutes under load)
	processingTimeout := time.Now().Add(-10 * time.Minute)

	processingQueues := []string{
		"zk_update_processing_queue",
		"zk_append_processing_queue",
		"zk_address_append_processing_queue",
	}

	totalRecovered := int64(0)
	totalFailed := int64(0)

	for _, queueName := range processingQueues {
		recovered, failed, err := rq.recoverStuckJobsFromQueue(queueName, processingTimeout)
		if err != nil {
			logging.Logger().Error().
				Err(err).
				Str("queue", queueName).
				Msg("Failed to recover stuck jobs from processing queue")
			continue
		}
		totalRecovered += recovered
		totalFailed += failed
	}

	if totalRecovered > 0 || totalFailed > 0 {
		logging.Logger().Info().
			Int64("recovered_jobs", totalRecovered).
			Int64("failed_jobs", totalFailed).
			Time("timeout_cutoff", processingTimeout).
			Msg("Processed stuck jobs from processing queues")
	}

	return nil
}

func (rq *RedisQueue) CleanupOldFailedJobs() error {
	cutoffTime := time.Now().Add(-1 * time.Hour)

	removed, err := rq.cleanupOldRequestsFromQueue("zk_failed_queue", cutoffTime)
	if err != nil {
		logging.Logger().Error().
			Err(err).
			Msg("Failed to cleanup old failed jobs")
		return err
	}

	if removed > 0 {
		logging.Logger().Info().
			Int64("removed_failed_jobs", removed).
			Time("cutoff_time", cutoffTime).
			Msg("Cleaned up old failed jobs")
	}

	return nil
}

func (rq *RedisQueue) recoverStuckJobsFromQueue(queueName string, timeoutCutoff time.Time) (int64, int64, error) {
	items, err := rq.Client.LRange(rq.Ctx, queueName, 0, -1).Result()
	if err != nil {
		return 0, 0, fmt.Errorf("failed to get processing queue items: %w", err)
	}

	var recoveredCount int64
	var failedCount int64

	for _, item := range items {
		var job ProofJob
		if json.Unmarshal([]byte(item), &job) == nil {
			if job.CreatedAt.Before(timeoutCutoff) {
				count, err := rq.Client.LRem(rq.Ctx, queueName, 1, item).Result()
				if err != nil {
					logging.Logger().Error().
						Err(err).
						Str("job_id", job.ID).
						Str("queue", queueName).
						Msg("Failed to remove stuck job from processing queue")
					continue
				}

				if count > 0 {
					originalJobID := job.ID
					if len(job.ID) > 11 && job.ID[len(job.ID)-11:] == "_processing" {
						originalJobID = job.ID[:len(job.ID)-11]
					}

					fiveMinutesAgo := time.Now().Add(-5 * time.Minute)
					if job.CreatedAt.Before(fiveMinutesAgo) {
						// Extract circuit type from payload for debugging, but don't store full payload
						// to prevent memory issues (payloads can be hundreds of KB)
						var circuitType string
						var payloadMeta map[string]interface{}
						if json.Unmarshal(job.Payload, &payloadMeta) == nil {
							if ct, ok := payloadMeta["circuitType"].(string); ok {
								circuitType = ct
							}
						}

						failureDetails := map[string]interface{}{
							"original_job": map[string]interface{}{
								"id":           originalJobID,
								"type":         "zk_proof",
								"circuit_type": circuitType,
								"payload_size": len(job.Payload),
								"created_at":   job.CreatedAt,
							},
							"error":     "Job timed out in processing queue (stuck for >5 minutes)",
							"failed_at": time.Now(),
							"timeout":   true,
						}

						failedData, _ := json.Marshal(failureDetails)
						failedJob := &ProofJob{
							ID:        originalJobID + "_failed",
							Type:      "failed",
							Payload:   json.RawMessage(failedData),
							CreatedAt: time.Now(),
						}

						err = rq.EnqueueProof("zk_failed_queue", failedJob)
						if err != nil {
							logging.Logger().Error().
								Err(err).
								Str("job_id", originalJobID).
								Msg("Failed to move timed out job to failed queue")
						} else {
							failedCount++
							logging.Logger().Warn().
								Str("job_id", originalJobID).
								Str("processing_queue", queueName).
								Time("stuck_since", job.CreatedAt).
								Msg("Moved timed out job to failed queue (processing timeout >5min)")
						}
					} else {
						originalQueue := getOriginalQueueFromProcessing(queueName)
						if originalQueue != "" {
							originalJob := &ProofJob{
								ID:        originalJobID,
								Type:      "zk_proof",
								Payload:   job.Payload,
								CreatedAt: job.CreatedAt,
							}

							err = rq.EnqueueProof(originalQueue, originalJob)
							if err != nil {
								logging.Logger().Error().
									Err(err).
									Str("job_id", originalJobID).
									Str("target_queue", originalQueue).
									Msg("Failed to recover stuck job")
							} else {
								recoveredCount++
								logging.Logger().Info().
									Str("job_id", originalJobID).
									Str("from_queue", queueName).
									Str("to_queue", originalQueue).
									Time("stuck_since", job.CreatedAt).
									Msg("Recovered stuck job")
							}
						}
					}
				}
			}
		}
	}

	return recoveredCount, failedCount, nil
}

func getOriginalQueueFromProcessing(processingQueueName string) string {
	switch processingQueueName {
	case "zk_update_processing_queue":
		return "zk_update_queue"
	case "zk_append_processing_queue":
		return "zk_append_queue"
	case "zk_address_append_processing_queue":
		return "zk_address_append_queue"
	default:
		return ""
	}
}

func (rq *RedisQueue) cleanupOldRequestsFromQueue(queueName string, cutoffTime time.Time) (int64, error) {
	items, err := rq.Client.LRange(rq.Ctx, queueName, 0, -1).Result()
	if err != nil {
		return 0, fmt.Errorf("failed to get queue items: %w", err)
	}

	var removedCount int64

	for _, item := range items {
		var job ProofJob
		if json.Unmarshal([]byte(item), &job) == nil {
			if job.CreatedAt.Before(cutoffTime) {
				// Remove this old job
				count, err := rq.Client.LRem(rq.Ctx, queueName, 1, item).Result()
				if err != nil {
					logging.Logger().Error().
						Err(err).
						Str("job_id", job.ID).
						Str("queue", queueName).
						Msg("Failed to remove old job")
					continue
				}
				if count > 0 {
					removedCount++

					// Also clean up the index entry if this is a results/failed queue
					rq.cleanupIndexEntry(queueName, job.ID)

					logging.Logger().Debug().
						Str("job_id", job.ID).
						Str("queue", queueName).
						Time("created_at", job.CreatedAt).
						Msg("Removed old proof request")
				}
			}
		}
	}

	return removedCount, nil
}

// cleanupIndexEntry removes the hash index entry for a job being cleaned up.
// Looks up the inputHash from zk_input_hash_{jobID} and removes from the appropriate index.
func (rq *RedisQueue) cleanupIndexEntry(queueName string, jobID string) {
	// Extract original job ID (remove _failed suffix if present)
	originalJobID := jobID
	if len(jobID) > 7 && jobID[len(jobID)-7:] == "_failed" {
		originalJobID = jobID[:len(jobID)-7]
	}

	// Look up the input hash for this job
	inputHash, err := rq.Client.Get(rq.Ctx, fmt.Sprintf("zk_input_hash_%s", originalJobID)).Result()
	if err != nil {
		// Input hash not found or expired - nothing to clean up
		return
	}

	// Remove from the appropriate index based on queue type
	switch queueName {
	case "zk_results_queue":
		rq.RemoveResultIndex(inputHash)
	case "zk_failed_queue":
		rq.RemoveFailureIndex(inputHash)
	}
}

// ComputeInputHash computes a SHA256 hash of the proof input payload
func ComputeInputHash(payload json.RawMessage) string {
	hash := sha256.Sum256(payload)
	return hex.EncodeToString(hash[:])
}

// FindCachedResult searches for a cached result by input hash.
// Returns the proof result (as ProofWithTiming) and job ID if found, otherwise returns nil.
func (rq *RedisQueue) FindCachedResult(inputHash string) (*common.ProofWithTiming, string, error) {
	jobID, err := rq.Client.HGet(rq.Ctx, ResultsIndexKey, inputHash).Result()
	if err == nil && jobID != "" {
		result, fetchErr := rq.Client.Get(rq.Ctx, fmt.Sprintf("zk_result_%s", jobID)).Result()
		if fetchErr == nil {
			var proofWithTiming common.ProofWithTiming
			if json.Unmarshal([]byte(result), &proofWithTiming) == nil {
				logging.Logger().Info().
					Str("input_hash", inputHash).
					Str("cached_job_id", jobID).
					Int64("proof_duration_ms", proofWithTiming.ProofDurationMs).
					Msg("Found cached successful proof result via index")
				return &proofWithTiming, jobID, nil
			}
		}
		// Index entry exists but result is missing/invalid - clean up stale index entry
		logging.Logger().Debug().
			Str("input_hash", inputHash).
			Str("job_id", jobID).
			Msg("Stale index entry, removing and falling back to queue scan")
		rq.RemoveResultIndex(inputHash)
	} else if err != nil && err != redis.Nil {
		logging.Logger().Warn().
			Err(err).
			Str("input_hash", inputHash).
			Msg("Error querying results index, falling back to queue scan")
	}

	// Fallback: O(n) queue scan for backward compatibility with unindexed results
	items, err := rq.Client.LRange(rq.Ctx, "zk_results_queue", 0, -1).Result()
	if err != nil {
		return nil, "", fmt.Errorf("failed to search results queue: %w", err)
	}

	for _, item := range items {
		var resultJob ProofJob
		if json.Unmarshal([]byte(item), &resultJob) == nil && resultJob.Type == "result" {
			// Check if this result has the same input hash
			storedHash, err := rq.Client.Get(rq.Ctx, fmt.Sprintf("zk_input_hash_%s", resultJob.ID)).Result()
			if err == nil && storedHash == inputHash {
				var proofWithTiming common.ProofWithTiming
				err = json.Unmarshal(resultJob.Payload, &proofWithTiming)
				if err != nil {
					logging.Logger().Warn().
						Err(err).
						Str("input_hash", inputHash).
						Str("job_id", resultJob.ID).
						Msg("Failed to unmarshal cached result payload, skipping")
					continue
				}

				logging.Logger().Info().
					Str("input_hash", inputHash).
					Str("cached_job_id", resultJob.ID).
					Int64("proof_duration_ms", proofWithTiming.ProofDurationMs).
					Msg("Found cached successful proof result via queue scan")

				rq.IndexResultByHash(inputHash, resultJob.ID)

				return &proofWithTiming, resultJob.ID, nil
			}
		}
	}

	return nil, "", nil
}

// FindCachedFailure searches for a cached failure by input hash.
// Returns the failure details and job ID if found, otherwise returns nil.
func (rq *RedisQueue) FindCachedFailure(inputHash string) (map[string]interface{}, string, error) {
	jobID, err := rq.Client.HGet(rq.Ctx, FailedIndexKey, inputHash).Result()
	if err == nil && jobID != "" {
		// Found in index, search for the job in failed queue by ID
		items, err := rq.Client.LRange(rq.Ctx, "zk_failed_queue", 0, -1).Result()
		if err == nil {
			failedJobID := jobID + "_failed"
			for _, item := range items {
				var failedJob ProofJob
				if json.Unmarshal([]byte(item), &failedJob) == nil && failedJob.ID == failedJobID {
					var failureDetails map[string]interface{}
					if json.Unmarshal(failedJob.Payload, &failureDetails) == nil {
						logging.Logger().Info().
							Str("input_hash", inputHash).
							Str("cached_job_id", jobID).
							Msg("Found cached failed proof result via index")
						return failureDetails, jobID, nil
					}
				}
			}
		}
		// Index entry exists but failure job not found - clean up stale index entry
		logging.Logger().Debug().
			Str("input_hash", inputHash).
			Str("job_id", jobID).
			Msg("Stale failure index entry, removing and falling back to queue scan")
		rq.RemoveFailureIndex(inputHash)
	} else if err != nil && err != redis.Nil {
		logging.Logger().Warn().
			Err(err).
			Str("input_hash", inputHash).
			Msg("Error querying failed index, falling back to queue scan")
	}

	// Fallback: O(n) queue scan for backward compatibility with unindexed failures
	items, err := rq.Client.LRange(rq.Ctx, "zk_failed_queue", 0, -1).Result()
	if err != nil {
		return nil, "", fmt.Errorf("failed to search failed queue: %w", err)
	}

	for _, item := range items {
		var failedJob ProofJob
		if json.Unmarshal([]byte(item), &failedJob) == nil && failedJob.Type == "failed" {
			// Extract the original job ID (remove _failed suffix)
			originalJobID := failedJob.ID
			if len(failedJob.ID) > 7 && failedJob.ID[len(failedJob.ID)-7:] == "_failed" {
				originalJobID = failedJob.ID[:len(failedJob.ID)-7]
			}

			// Check if this failure has the same input hash
			storedHash, err := rq.Client.Get(rq.Ctx, fmt.Sprintf("zk_input_hash_%s", originalJobID)).Result()
			if err == nil && storedHash == inputHash {
				// Found a matching failure
				var failureDetails map[string]interface{}
				err = json.Unmarshal(failedJob.Payload, &failureDetails)
				if err != nil {
					continue
				}

				logging.Logger().Info().
					Str("input_hash", inputHash).
					Str("cached_job_id", originalJobID).
					Msg("Found cached failed proof result via queue scan")

				// Backfill the index for future O(1) lookups
				rq.IndexFailureByHash(inputHash, originalJobID)

				return failureDetails, originalJobID, nil
			}
		}
	}

	return nil, "", nil
}

// StoreInputHash stores the input hash for a job ID to enable deduplication
func (rq *RedisQueue) StoreInputHash(jobID string, inputHash string) error {
	key := fmt.Sprintf("zk_input_hash_%s", jobID)
	err := rq.Client.Set(rq.Ctx, key, inputHash, 1*time.Hour).Err()
	if err != nil {
		return fmt.Errorf("failed to store input hash: %w", err)
	}

	logging.Logger().Debug().
		Str("job_id", jobID).
		Str("input_hash", inputHash).
		Msg("Stored input hash for deduplication")

	return nil
}

// GetOrSetInFlightJob atomically checks if a job with the given input hash is already in-flight.
// If not, it registers the new job ID. Returns the existing job ID if found, or the new job ID if set.
// The isNew return value indicates whether this is a new job (true) or an existing one (false).
// TTL is set to 10 minutes to match the forester's max wait time.
func (rq *RedisQueue) GetOrSetInFlightJob(inputHash, jobID string) (existingJobID string, isNew bool, err error) {
	key := fmt.Sprintf("zk_inflight_%s", inputHash)

	// Try to set the key atomically - only succeeds if key doesn't exist
	set, err := rq.Client.SetNX(rq.Ctx, key, jobID, 10*time.Minute).Result()
	if err != nil {
		return "", false, fmt.Errorf("failed to check/set in-flight job: %w", err)
	}

	if set {
		// Key was set - this is a new job
		// Also store reverse mapping so we can find the input hash from job ID
		// This is needed for CleanupStaleInFlightMarker when job_not_found
		reverseKey := fmt.Sprintf("zk_input_hash_%s", jobID)
		rq.Client.Set(rq.Ctx, reverseKey, inputHash, 10*time.Minute)

		logging.Logger().Debug().
			Str("job_id", jobID).
			Str("input_hash", inputHash).
			Msg("Registered new in-flight job")
		return jobID, true, nil
	}

	// Key already exists - get the existing job ID
	existing, err := rq.Client.Get(rq.Ctx, key).Result()
	if err != nil {
		// Key might have expired between SetNX and Get - retry
		if err == redis.Nil {
			ok, err := rq.Client.SetNX(rq.Ctx, key, jobID, 10*time.Minute).Result()
			if err != nil {
				return "", false, fmt.Errorf("failed to set in-flight job on retry: %w", err)
			}
			if !ok {
				// Another worker won the race - fetch their job ID
				existing, err := rq.Client.Get(rq.Ctx, key).Result()
				if err != nil {
					return "", false, fmt.Errorf("failed to get winning job after retry race: %w", err)
				}
				return existing, false, nil
			}
			// We won the retry - store reverse mapping for cleanup
			reverseKey := fmt.Sprintf("zk_input_hash_%s", jobID)
			rq.Client.Set(rq.Ctx, reverseKey, inputHash, 10*time.Minute)
			return jobID, true, nil
		}
		return "", false, fmt.Errorf("failed to get existing in-flight job: %w", err)
	}

	logging.Logger().Info().
		Str("existing_job_id", existing).
		Str("input_hash", inputHash).
		Msg("Found existing in-flight job with same input")

	return existing, false, nil
}

// DeleteInFlightJob removes the in-flight marker for a job when it completes.
// This should be called when a job finishes (success or failure) to allow
// new jobs with the same input to be queued.
func (rq *RedisQueue) DeleteInFlightJob(inputHash, jobID string) error {
	key := fmt.Sprintf("zk_inflight_%s", inputHash)
	err := rq.Client.Del(rq.Ctx, key).Err()
	if err != nil {
		return fmt.Errorf("failed to delete in-flight job marker: %w", err)
	}

	// Also clean up the reverse mapping
	reverseKey := fmt.Sprintf("zk_input_hash_%s", jobID)
	rq.Client.Del(rq.Ctx, reverseKey)

	logging.Logger().Debug().
		Str("input_hash", inputHash).
		Str("job_id", jobID).
		Msg("Deleted in-flight job marker")

	return nil
}

// SetInFlightJob sets the in-flight marker for a job, replacing any existing marker.
// This is used when recovering from a stale marker to register a new job.
// Also sets the reverse mapping (jobID → inputHash) for cleanup.
func (rq *RedisQueue) SetInFlightJob(inputHash, jobID string, ttl time.Duration) error {
	key := fmt.Sprintf("zk_inflight_%s", inputHash)
	err := rq.Client.Set(rq.Ctx, key, jobID, ttl).Err()
	if err != nil {
		return fmt.Errorf("failed to set in-flight job marker: %w", err)
	}

	// Also store reverse mapping so we can find the input hash from job ID
	reverseKey := fmt.Sprintf("zk_input_hash_%s", jobID)
	if reverseErr := rq.Client.Set(rq.Ctx, reverseKey, inputHash, ttl).Err(); reverseErr != nil {
		logging.Logger().Warn().
			Err(reverseErr).
			Str("job_id", jobID).
			Msg("Failed to set reverse mapping (non-critical)")
	}

	logging.Logger().Debug().
		Str("input_hash", inputHash).
		Str("job_id", jobID).
		Dur("ttl", ttl).
		Msg("Set in-flight job marker")

	return nil
}

// CleanupStaleInFlightMarker removes a stale in-flight marker for a job that no longer exists.
// This is called when a status check returns job_not_found, indicating the job was lost
// (e.g., due to prover restart) but the in-flight marker still exists.
// This allows new requests with the same input to create a new job instead of being
// deduplicated to the stale job ID.
func (rq *RedisQueue) CleanupStaleInFlightMarker(jobID string) {
	// Get the input hash associated with this job ID
	inputHashKey := fmt.Sprintf("zk_input_hash_%s", jobID)
	inputHash, err := rq.Client.Get(rq.Ctx, inputHashKey).Result()
	if err != nil {
		// No input hash found - nothing to clean up
		return
	}

	// Check if the in-flight marker points to this job ID
	inFlightKey := fmt.Sprintf("zk_inflight_%s", inputHash)
	storedJobID, err := rq.Client.Get(rq.Ctx, inFlightKey).Result()
	if err != nil {
		// No in-flight marker - nothing to clean up
		return
	}

	// Only delete if this marker points to the stale job
	if storedJobID == jobID {
		rq.Client.Del(rq.Ctx, inFlightKey)
		logging.Logger().Info().
			Str("job_id", jobID).
			Str("input_hash", inputHash).
			Msg("Cleaned up stale in-flight marker for lost job")
	}

	// Also clean up the input hash mapping
	rq.Client.Del(rq.Ctx, inputHashKey)
}

// DeduplicationResult contains the result of a job deduplication check.
type DeduplicationResult struct {
	// JobID is the resolved job ID to use (either new or existing).
	JobID string
	// IsNew indicates this is a new job that needs to be enqueued.
	IsNew bool
	// IsDeduplicated indicates the request was deduplicated to an existing job.
	IsDeduplicated bool
	// StaleJobID is set when a stale job was found and cleaned up.
	StaleJobID string
}

// DeduplicateJob checks for an existing in-flight job with the same input hash.
// If an existing job is found and still valid (has result or metadata), it returns
// that job's ID with IsDeduplicated=true. If an existing marker points to a stale
// job (no result/metadata), it cleans up the stale marker and creates a new job.
// Returns the resolved job ID and flags indicating the deduplication outcome.
//
// The TTL for in-flight markers is 10 minutes to match the forester's max wait time.
func (rq *RedisQueue) DeduplicateJob(inputHash string) (*DeduplicationResult, error) {
	// Generate a new job ID
	newJobID := uuid.New().String()

	// Try to atomically set our job as in-flight
	existingJobID, isNew, err := rq.GetOrSetInFlightJob(inputHash, newJobID)
	if err != nil {
		logging.Logger().Warn().
			Err(err).
			Str("input_hash", inputHash).
			Msg("Failed to check for in-flight job, proceeding with new job")
		// On error, proceed with the new job
		return &DeduplicationResult{
			JobID: newJobID,
			IsNew: true,
		}, nil
	}

	// If we successfully set a new job, we're done
	if isNew {
		return &DeduplicationResult{
			JobID: existingJobID, // This is our newJobID
			IsNew: true,
		}, nil
	}

	// An existing job was found - verify it actually exists
	jobExists := false
	if result, _ := rq.GetResult(existingJobID); result != nil {
		jobExists = true
	} else if jobMeta, _ := rq.GetJobMeta(existingJobID); jobMeta != nil {
		jobExists = true
	}

	if jobExists {
		// Valid existing job found - deduplicate to it
		logging.Logger().Info().
			Str("existing_job_id", existingJobID).
			Str("input_hash", inputHash).
			Msg("Deduplicated proof request to existing job")

		return &DeduplicationResult{
			JobID:          existingJobID,
			IsNew:          false,
			IsDeduplicated: true,
		}, nil
	}

	// Job doesn't exist - stale marker found
	logging.Logger().Warn().
		Str("stale_job_id", existingJobID).
		Str("input_hash", inputHash).
		Msg("Deduplication found stale job ID - cleaning up and creating new job")

	// Clean up the stale marker
	rq.CleanupStaleInFlightMarker(existingJobID)

	// Generate a fresh job ID and set new in-flight marker
	freshJobID := uuid.New().String()
	if err := rq.SetInFlightJob(inputHash, freshJobID, 10*time.Minute); err != nil {
		return nil, fmt.Errorf("failed to set in-flight marker after stale cleanup: %w", err)
	}

	return &DeduplicationResult{
		JobID:      freshJobID,
		IsNew:      true,
		StaleJobID: existingJobID,
	}, nil
}
