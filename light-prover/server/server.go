package server

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"light/light-prover/logging"
	"light/light-prover/prover"
	"net/http"

	"github.com/gorilla/handlers"
	//"github.com/prometheus/client_golang/prometheus/promhttp"
)

type Error struct {
	StatusCode int
	Code       string
	Message    string
}

func malformedBodyError(err error) *Error {
	return &Error{StatusCode: http.StatusBadRequest, Code: "malformed_body", Message: err.Error()}
}

func provingError(err error) *Error {
	return &Error{StatusCode: http.StatusBadRequest, Code: "proving_error", Message: err.Error()}
}

func unexpectedError(err error) *Error {
	return &Error{StatusCode: http.StatusInternalServerError, Code: "unexpected_error", Message: err.Error()}
}

func (error *Error) MarshalJSON() ([]byte, error) {
	return json.Marshal(map[string]string{
		"code":    error.Code,
		"message": error.Message,
	})
}

func (error *Error) send(w http.ResponseWriter) {
	w.WriteHeader(error.StatusCode)
	jsonBytes, err := error.MarshalJSON()
	if err != nil {
		jsonBytes = []byte(`{"code": "unexpected_error", "message": "failed to marshal error"}`)
	}
	length, err := w.Write(jsonBytes)
	if err != nil || length != len(jsonBytes) {
		logging.Logger().Error().Err(err).Msg("error writing response")
	}
}

type Config struct {
	ProverAddress  string
	MetricsAddress string
}

func spawnServerJob(server *http.Server, label string) RunningJob {
	start := func() {
		err := server.ListenAndServe()
		if err != nil && !errors.Is(err, http.ErrServerClosed) {
			panic(fmt.Sprintf("%s failed: %s", label, err))
		}
	}
	shutdown := func() {
		logging.Logger().Info().Msgf("shutting down %s", label)
		err := server.Shutdown(context.Background())
		if err != nil {
			logging.Logger().Error().Err(err).Msgf("error when shutting down %s", label)
		}
		logging.Logger().Info().Msgf("%s shut down", label)
	}
	return SpawnJob(start, shutdown)
}

func Run(config *Config, provingSystem []*prover.ProvingSystem) RunningJob {
	metricsMux := http.NewServeMux()
	// TODO: Add metrics
	//metricsMux.Handle("/metrics", promhttp.Handler())
	metricsServer := &http.Server{Addr: config.MetricsAddress, Handler: metricsMux}
	metricsJob := spawnServerJob(metricsServer, "metrics server")
	logging.Logger().Info().Str("addr", config.MetricsAddress).Msg("metrics server started")

	proverMux := http.NewServeMux()
	proverMux.Handle("/prove", proveHandler{provingSystem: provingSystem})
	proverMux.Handle("/health", healthHandler{})

	// Setup CORS
	// TODO: Enforce strict CORS policy
	corsHandler := handlers.CORS(
		handlers.AllowedHeaders([]string{"X-Requested-With", "Content-Type", "Authorization"}),
		handlers.AllowedOrigins([]string{"*"}),
		handlers.AllowedMethods([]string{"GET", "POST", "PUT", "DELETE", "OPTIONS"}),
	)

	proverServer := &http.Server{Addr: config.ProverAddress, Handler: corsHandler(proverMux)}
	proverJob := spawnServerJob(proverServer, "prover server")
	logging.Logger().Info().Str("addr", config.ProverAddress).Msg("app server started")

	return CombineJobs(metricsJob, proverJob)
}

type proveHandler struct {
	provingSystem []*prover.ProvingSystem
}

type healthHandler struct {
}

func (handler proveHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	logging.Logger().Info().Msg("received prove request")
	buf, err := io.ReadAll(r.Body)
	if err != nil {
		logging.Logger().Info().Msg("error reading request body")
		logging.Logger().Info().Msg(err.Error())
		malformedBodyError(err).send(w)
		return
	}

	var circuitType prover.CircuitType

	circuitType, err = prover.ParseCircuitType(buf)
	if err != nil {
		logging.Logger().Info().Msg("error parsing circuit type")
		logging.Logger().Info().Msg(err.Error())
		malformedBodyError(err).send(w)
		return
	}

	var proof *prover.Proof
	var proofError *Error
	if circuitType == prover.Inclusion {
		proof, proofError = handler.inclusionProof(buf)
	}
	if circuitType == prover.NonInclusion {
		proof, proofError = handler.nonInclusionProof(buf)
	}
	if circuitType == prover.Combined {
		proof, proofError = handler.combinedProof(buf)
	}

	if proofError != nil {
		println(proofError.Message)
		logging.Logger().Err(err)
		proofError.send(w)
		return
	}

	responseBytes, err := json.Marshal(&proof)
	if err != nil {
		logging.Logger().Err(err)
		unexpectedError(err).send(w)
		return
	}

	w.WriteHeader(http.StatusOK)
	_, err = w.Write(responseBytes)

	if err != nil {
		logging.Logger().Err(err)
	}
}

func (handler proveHandler) inclusionProof(buf []byte) (*prover.Proof, *Error) {
	var proof *prover.Proof
	var params prover.InclusionParameters

	var err = json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)

	}

	var numberOfCompressedAccounts = uint32(len(params.Inputs))

	var ps *prover.ProvingSystem
	for _, provingSystem := range handler.provingSystem {
		if provingSystem.InclusionNumberOfCompressedAccounts == numberOfCompressedAccounts {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for %d compressedAccounts", numberOfCompressedAccounts))
	}

	proof, err = ps.ProveInclusion(&params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) nonInclusionProof(buf []byte) (*prover.Proof, *Error) {
	var proof *prover.Proof
	var params prover.NonInclusionParameters

	var err = json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)
	}

	var numberOfCompressedAccounts = uint32(len(params.Inputs))
	var ps *prover.ProvingSystem
	for _, provingSystem := range handler.provingSystem {
		if provingSystem.NonInclusionNumberOfCompressedAccounts == numberOfCompressedAccounts {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for %d compressedAccounts", numberOfCompressedAccounts))
	}

	proof, err = ps.ProveNonInclusion(&params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) combinedProof(buf []byte) (*prover.Proof, *Error) {
	var proof *prover.Proof
	var params prover.CombinedParameters

	var err = json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)

	}

	var inclusionNumberOfCompressedAccounts = uint32(len(params.InclusionParameters.Inputs))
	var nonInclusionNumberOfCompressedAccounts = uint32(len(params.NonInclusionParameters.Inputs))

	var ps *prover.ProvingSystem
	for _, provingSystem := range handler.provingSystem {
		if provingSystem.InclusionNumberOfCompressedAccounts == inclusionNumberOfCompressedAccounts && provingSystem.NonInclusionNumberOfCompressedAccounts == nonInclusionNumberOfCompressedAccounts {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for %d inclusion compressedAccounts & %d non-inclusion", inclusionNumberOfCompressedAccounts, nonInclusionNumberOfCompressedAccounts))
	}
	proof, err = ps.ProveCombined(&params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler healthHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	logging.Logger().Info().Msg("received health check request")
	responseBytes, err := json.Marshal(map[string]string{"status": "ok"})
	w.WriteHeader(http.StatusOK)
	_, err = w.Write(responseBytes)
	if err != nil {
		logging.Logger().Error().Err(err).Msg("error writing response")
	}
}
