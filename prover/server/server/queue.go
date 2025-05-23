package server

import (
	"context"
	"encoding/json"
	"fmt"
	"light/light-prover/logging"
	"light/light-prover/prover"
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

	queues := []string{"zk_proof_queue", "zk_priority_queue", "zk_processing_queue", "zk_failed_queue", "zk_results_queue"}

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

func (rq *RedisQueue) GetResult(jobID string) (interface{}, error) {
	key := fmt.Sprintf("zk_result_%s", jobID)
	result, err := rq.Client.Get(rq.Ctx, key).Result()
	if err == nil {
		var proof prover.Proof
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
				var proof prover.Proof
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
	err = rq.Client.Set(rq.Ctx, key, resultData, 24*time.Hour).Err()
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
			Msg("Cleaned up old results from queue")
	}

	return nil
}
