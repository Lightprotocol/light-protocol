package server

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover/common"
	"net/http"
	"os"
	"time"
)

// SP1ProxyConfig holds configuration for SP1 prover server integration.
type SP1ProxyConfig struct {
	// ServerURL is the base URL of the SP1 prover server (e.g., "http://localhost:3002")
	ServerURL string
	// Enabled determines whether to route batch operations to SP1
	Enabled bool
	// Timeout for SP1 proof requests
	Timeout time.Duration
}

// GetSP1ConfigFromEnv reads SP1 configuration from environment variables.
func GetSP1ConfigFromEnv() *SP1ProxyConfig {
	serverURL := os.Getenv("SP1_PROVER_URL")
	if serverURL == "" {
		return &SP1ProxyConfig{Enabled: false}
	}

	timeout := 600 * time.Second // Default 10 minutes for Groth16 wrapping
	if timeoutStr := os.Getenv("SP1_PROVER_TIMEOUT"); timeoutStr != "" {
		if d, err := time.ParseDuration(timeoutStr); err == nil {
			timeout = d
		}
	}

	return &SP1ProxyConfig{
		ServerURL: serverURL,
		Enabled:   true,
		Timeout:   timeout,
	}
}

// SP1ProxyClient handles forwarding proof requests to the SP1 prover server.
type SP1ProxyClient struct {
	config     *SP1ProxyConfig
	httpClient *http.Client
}

// NewSP1ProxyClient creates a new SP1 proxy client.
func NewSP1ProxyClient(config *SP1ProxyConfig) *SP1ProxyClient {
	return &SP1ProxyClient{
		config: config,
		httpClient: &http.Client{
			Timeout: config.Timeout,
		},
	}
}

// ProxyBatchProof forwards a batch proof request to the SP1 prover server.
// Returns the proof if successful, or nil if SP1 is not configured/available.
func (c *SP1ProxyClient) ProxyBatchProof(circuitType common.CircuitType, requestBody []byte) (*common.Proof, error) {
	if !c.config.Enabled {
		return nil, fmt.Errorf("SP1 proxy not enabled")
	}

	// Only proxy batch operations
	if !isBatchCircuitType(circuitType) {
		return nil, fmt.Errorf("circuit type %s is not a batch operation", circuitType)
	}

	url := fmt.Sprintf("%s/prove", c.config.ServerURL)

	logging.Logger().Info().
		Str("circuit_type", string(circuitType)).
		Str("sp1_url", url).
		Msg("Forwarding batch proof request to SP1 prover")

	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(requestBody))
	if err != nil {
		return nil, fmt.Errorf("failed to create SP1 request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	startTime := time.Now()
	resp, err := c.httpClient.Do(req)
	if err != nil {
		logging.Logger().Error().
			Err(err).
			Str("circuit_type", string(circuitType)).
			Msg("SP1 prover request failed")
		return nil, fmt.Errorf("SP1 prover request failed: %w", err)
	}
	defer resp.Body.Close()

	duration := time.Since(startTime)

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read SP1 response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		logging.Logger().Error().
			Int("status_code", resp.StatusCode).
			Str("body", string(body)).
			Str("circuit_type", string(circuitType)).
			Msg("SP1 prover returned error")
		return nil, fmt.Errorf("SP1 prover returned status %d: %s", resp.StatusCode, string(body))
	}

	// Parse SP1 response
	var sp1Response struct {
		Proof struct {
			Ar  []string   `json:"ar"`
			Bs  [][]string `json:"bs"`
			Krs []string   `json:"krs"`
		} `json:"proof"`
		ProofDurationMs int64 `json:"proof_duration_ms"`
	}

	if err := json.Unmarshal(body, &sp1Response); err != nil {
		return nil, fmt.Errorf("failed to parse SP1 response: %w", err)
	}

	logging.Logger().Info().
		Str("circuit_type", string(circuitType)).
		Int64("sp1_duration_ms", sp1Response.ProofDurationMs).
		Int64("total_duration_ms", duration.Milliseconds()).
		Msg("SP1 proof generation completed")

	// Convert SP1 response to common.Proof format
	// The SP1 Groth16 proof format should match Gnark's format
	proof, err := convertSP1ProofToCommon(sp1Response.Proof.Ar, sp1Response.Proof.Bs, sp1Response.Proof.Krs)
	if err != nil {
		return nil, fmt.Errorf("failed to convert SP1 proof: %w", err)
	}

	return proof, nil
}

// HealthCheck verifies the SP1 prover server is reachable.
func (c *SP1ProxyClient) HealthCheck() error {
	if !c.config.Enabled {
		return fmt.Errorf("SP1 proxy not enabled")
	}

	url := fmt.Sprintf("%s/health", c.config.ServerURL)
	resp, err := c.httpClient.Get(url)
	if err != nil {
		return fmt.Errorf("SP1 health check failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("SP1 health check returned status %d", resp.StatusCode)
	}

	return nil
}

func isBatchCircuitType(circuitType common.CircuitType) bool {
	switch circuitType {
	case common.BatchAppendCircuitType,
		common.BatchUpdateCircuitType,
		common.BatchAddressAppendCircuitType:
		return true
	default:
		return false
	}
}

// convertSP1ProofToCommon converts SP1's Groth16 proof format to the common.Proof format.
// Both use BN254 curve, so the proof points should be directly compatible.
func convertSP1ProofToCommon(ar []string, bs [][]string, krs []string) (*common.Proof, error) {
	// Validate input lengths
	if len(ar) != 2 {
		return nil, fmt.Errorf("invalid ar length: expected 2, got %d", len(ar))
	}
	if len(bs) != 2 || len(bs[0]) != 2 || len(bs[1]) != 2 {
		return nil, fmt.Errorf("invalid bs structure: expected [[2],[2]]")
	}
	if len(krs) != 2 {
		return nil, fmt.Errorf("invalid krs length: expected 2, got %d", len(krs))
	}

	// Create ProofJSON structure matching common.ProofJSON
	proofJSON := struct {
		Ar  [2]string    `json:"ar"`
		Bs  [2][2]string `json:"bs"`
		Krs [2]string    `json:"krs"`
	}{
		Ar:  [2]string{ar[0], ar[1]},
		Bs:  [2][2]string{{bs[0][0], bs[0][1]}, {bs[1][0], bs[1][1]}},
		Krs: [2]string{krs[0], krs[1]},
	}

	// Serialize to JSON
	jsonBytes, err := json.Marshal(proofJSON)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal proof JSON: %w", err)
	}

	// Use common.Proof's UnmarshalJSON to create the gnark proof
	proof := &common.Proof{}
	if err := proof.UnmarshalJSON(jsonBytes); err != nil {
		return nil, fmt.Errorf("failed to unmarshal SP1 proof: %w", err)
	}

	return proof, nil
}
