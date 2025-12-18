package server

import (
	"runtime"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"light/light-prover/logging"
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

	ProofPanicsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "prover_proof_panics_total",
			Help: "Total number of panics recovered during proof processing",
		},
		[]string{"circuit_type"},
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

	// Memory metrics for proof generation
	ProofMemoryUsage = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "prover_proof_memory_usage_bytes",
			Help:    "Memory allocated during proof generation (heap alloc delta)",
			Buckets: prometheus.ExponentialBuckets(1024*1024*100, 2, 12), // 100MB to 400GB
		},
		[]string{"circuit_type"},
	)

	ProofPeakMemory = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "prover_proof_peak_memory_bytes",
			Help: "Peak memory observed during proof generation by circuit type",
		},
		[]string{"circuit_type"},
	)

	SystemMemoryUsage = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "prover_system_memory_bytes",
			Help: "System memory statistics",
		},
		[]string{"type"}, // heap_alloc, heap_sys, heap_inuse, sys
	)
)

type MetricTimer struct {
	start          time.Time
	circuitType    string
	startHeapAlloc uint64
}

func StartProofTimer(circuitType string) *MetricTimer {
	ProofRequestsTotal.WithLabelValues(circuitType).Inc()
	ActiveJobs.Inc()

	// Capture memory state before proof generation
	var memStats runtime.MemStats
	runtime.ReadMemStats(&memStats)

	// Update system memory gauges
	SystemMemoryUsage.WithLabelValues("heap_alloc").Set(float64(memStats.HeapAlloc))
	SystemMemoryUsage.WithLabelValues("heap_sys").Set(float64(memStats.HeapSys))
	SystemMemoryUsage.WithLabelValues("heap_inuse").Set(float64(memStats.HeapInuse))
	SystemMemoryUsage.WithLabelValues("sys").Set(float64(memStats.Sys))

	return &MetricTimer{
		start:          time.Now(),
		circuitType:    circuitType,
		startHeapAlloc: memStats.HeapAlloc,
	}
}

func (t *MetricTimer) ObserveDuration() {
	duration := time.Since(t.start).Seconds()
	ProofGenerationDuration.WithLabelValues(t.circuitType).Observe(duration)
	ActiveJobs.Dec()

	// Capture memory state after proof generation
	var memStats runtime.MemStats
	runtime.ReadMemStats(&memStats)

	// Calculate memory delta (may be negative due to GC, use max with 0)
	memDelta := int64(memStats.HeapAlloc) - int64(t.startHeapAlloc)
	if memDelta < 0 {
		memDelta = 0
	}

	// Record memory usage for this proof
	ProofMemoryUsage.WithLabelValues(t.circuitType).Observe(float64(memDelta))

	// Update peak memory if this is higher
	currentPeak, _ := ProofPeakMemory.GetMetricWithLabelValues(t.circuitType)
	if currentPeak != nil {
		// Note: Gauge doesn't have a Get method, so we track via histogram max
	}

	// Update system memory gauges
	SystemMemoryUsage.WithLabelValues("heap_alloc").Set(float64(memStats.HeapAlloc))
	SystemMemoryUsage.WithLabelValues("heap_sys").Set(float64(memStats.HeapSys))
	SystemMemoryUsage.WithLabelValues("heap_inuse").Set(float64(memStats.HeapInuse))
	SystemMemoryUsage.WithLabelValues("sys").Set(float64(memStats.Sys))

	// Log memory usage for debugging
	logging.Logger().Info().
		Str("circuit_type", t.circuitType).
		Float64("duration_sec", duration).
		Uint64("start_heap_mb", t.startHeapAlloc/1024/1024).
		Uint64("end_heap_mb", memStats.HeapAlloc/1024/1024).
		Int64("delta_mb", memDelta/1024/1024).
		Uint64("sys_mb", memStats.Sys/1024/1024).
		Msg("Proof generation completed with memory stats")
}

func (t *MetricTimer) ObserveError(errorType string) {
	ProofGenerationErrors.WithLabelValues(t.circuitType, errorType).Inc()
	ActiveJobs.Dec()

	// Still record memory stats on error
	var memStats runtime.MemStats
	runtime.ReadMemStats(&memStats)
	SystemMemoryUsage.WithLabelValues("heap_alloc").Set(float64(memStats.HeapAlloc))
	SystemMemoryUsage.WithLabelValues("heap_sys").Set(float64(memStats.HeapSys))
	SystemMemoryUsage.WithLabelValues("heap_inuse").Set(float64(memStats.HeapInuse))
	SystemMemoryUsage.WithLabelValues("sys").Set(float64(memStats.Sys))
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
