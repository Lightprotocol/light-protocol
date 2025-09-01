package server

import (
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var (
	ProofRequestsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "prover_proof_requests_total",
			Help: "Total number of proof generation requests by circuit type",
		},
		[]string{"circuit_type"},
	)

	ProofGenerationDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "prover_proof_generation_duration_seconds",
			Help:    "Duration of proof generation in seconds",
			Buckets: prometheus.ExponentialBuckets(0.1, 2, 15),
		},
		[]string{"circuit_type"},
	)

	ProofGenerationErrors = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "prover_proof_generation_errors_total",
			Help: "Total number of proof generation errors by circuit type",
		},
		[]string{"circuit_type", "error_type"},
	)

	QueueWaitTime = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "prover_queue_wait_time_seconds",
			Help:    "Time spent waiting in queue before processing",
			Buckets: prometheus.ExponentialBuckets(0.1, 2, 12),
		},
		[]string{"circuit_type"},
	)

	JobsProcessed = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "prover_jobs_processed_total",
			Help: "Total number of jobs processed",
		},
		[]string{"status"},
	)

	ExpiredJobsCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "prover_expired_jobs_total",
			Help: "Total number of expired jobs that were skipped",
		},
		[]string{"queue"},
	)

	ActiveJobs = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "prover_active_jobs",
			Help: "Number of currently active proof generation jobs",
		},
	)

	CircuitInputSize = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "prover_circuit_input_size_bytes",
			Help:    "Size of circuit inputs in bytes",
			Buckets: prometheus.ExponentialBuckets(1024, 2, 15),
		},
		[]string{"circuit_type"},
	)

	CircuitProofSize = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "prover_circuit_proof_size_bytes",
			Help:    "Size of generated proofs in bytes",
			Buckets: prometheus.ExponentialBuckets(256, 2, 10),
		},
		[]string{"circuit_type"},
	)
)

type MetricTimer struct {
	start       time.Time
	circuitType string
}

func StartProofTimer(circuitType string) *MetricTimer {
	ProofRequestsTotal.WithLabelValues(circuitType).Inc()
	ActiveJobs.Inc()
	return &MetricTimer{
		start:       time.Now(),
		circuitType: circuitType,
	}
}

func (t *MetricTimer) ObserveDuration() {
	duration := time.Since(t.start).Seconds()
	ProofGenerationDuration.WithLabelValues(t.circuitType).Observe(duration)
	ActiveJobs.Dec()
}

func (t *MetricTimer) ObserveError(errorType string) {
	ProofGenerationErrors.WithLabelValues(t.circuitType, errorType).Inc()
	ActiveJobs.Dec()
}

func RecordJobComplete(success bool) {
	if success {
		JobsProcessed.WithLabelValues("completed").Inc()
	} else {
		JobsProcessed.WithLabelValues("failed").Inc()
	}
}

func RecordCircuitInputSize(circuitType string, sizeBytes int) {
	CircuitInputSize.WithLabelValues(circuitType).Observe(float64(sizeBytes))
}

func RecordProofSize(circuitType string, sizeBytes int) {
	CircuitProofSize.WithLabelValues(circuitType).Observe(float64(sizeBytes))
}
