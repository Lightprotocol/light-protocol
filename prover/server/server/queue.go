package server

import (
	"context"
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"time"

	"github.com/redis/go-redis/v9"
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

	client := redis.NewClient(opts)
	ctx := context.Background()

	ctx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()

	if err := client.Ping(ctx).Err(); err != nil {
		return nil, fmt.Errorf("failed to connect to Redis: %w", err)
	}

	return &RedisQueue{Client: client, Ctx: context.Background()}, nil
}

func (rq *RedisQueue) EnqueueProof(queueName string, job *ProofJob) error {
	data, err := json.Marshal(job)
	if err != nil {
		return fmt.Errorf("failed to marshal job: %w", err)
	}

	err = rq.Client.RPush(rq.Ctx, queueName, data).Err()
	if err != nil {
		return fmt.Errorf("failed to enqueue job: %w", err)
	}

	logging.Logger().Info().
		Str("job_id", job.ID).
		Str("queue", queueName).
		Msg("Job enqueued successfully")
	return nil
}

func (rq *RedisQueue) DequeueProof(queueName string, timeout time.Duration) (*ProofJob, error) {
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
		var proof common.Proof
		err = json.Unmarshal([]byte(result), &proof)
		if err != nil {
			logging.Logger().Error().
				Str("job_id", jobID).
				Err(err).
				Str("result", result).
				Msg("Failed to unmarshal result")

			return nil, fmt.Errorf("failed to unmarshal direct result: %w", err)
		}
		return &proof, nil
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
				var proof common.Proof
				err = json.Unmarshal(resultJob.Payload, &proof)
				if err != nil {
					return nil, fmt.Errorf("failed to unmarshal queued result: %w", err)
				}
				rq.StoreResult(jobID, &proof)

				return &proof, nil
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

func (rq *RedisQueue) CleanupOldResults() error {
	// Remove successful results older than 1 hour
	cutoffTime := time.Now().Add(-1 * time.Hour)

	removed, err := rq.cleanupOldRequestsFromQueue("zk_results_queue", cutoffTime)
	if err != nil {
		logging.Logger().Error().
			Err(err).
			Msg("Failed to cleanup old results by time")
	}

	if removed > 0 {
		logging.Logger().Info().
			Int64("removed_results", removed).
			Time("cutoff_time", cutoffTime).
			Msg("Cleaned up old results by time")
	}

	ctx := context.Background()
	length, err := rq.Client.LLen(ctx, "zk_results_queue").Result()
	if err != nil {
		return err
	}

	if length > 1000 {
		toRemove := length - 1000
		for i := int64(0); i < toRemove; i++ {
			rq.Client.LPop(ctx, "zk_results_queue")
		}

		logging.Logger().Info().
			Int64("removed_items", toRemove).
			Msg("Cleaned up old results from queue (length-based safety)")
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
		removed, err := rq.cleanupOldRequestsFromQueue(queueName, cutoffTime)
		if err != nil {
			logging.Logger().Error().
				Err(err).
				Str("queue", queueName).
				Msg("Failed to cleanup old requests from queue")
			continue
		}
		totalRemoved += removed
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
	// Jobs stuck in processing for more than 2 minutes are considered stuck
	processingTimeout := time.Now().Add(-2 * time.Minute)

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
						failureDetails := map[string]interface{}{
							"original_job": map[string]interface{}{
								"id":         originalJobID,
								"type":       "zk_proof",
								"payload":    job.Payload,
								"created_at": job.CreatedAt,
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
