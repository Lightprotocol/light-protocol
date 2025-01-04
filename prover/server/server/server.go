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

func Run(config *Config, provingSystemsV1 []*prover.ProvingSystemV1, provingSystemsV2 []*prover.ProvingSystemV2) RunningJob {
	metricsMux := http.NewServeMux()
	// TODO: Add metrics
	//metricsMux.Handle("/metrics", promhttp.Handler())
	metricsServer := &http.Server{Addr: config.MetricsAddress, Handler: metricsMux}
	metricsJob := spawnServerJob(metricsServer, "metrics server")
	logging.Logger().Info().Str("addr", config.MetricsAddress).Msg("metrics server started")

	proverMux := http.NewServeMux()
	proverMux.Handle("/prove", proveHandler{
		provingSystemsV1: provingSystemsV1,
		provingSystemsV2: provingSystemsV2,
	})
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
	provingSystemsV1 []*prover.ProvingSystemV1
	provingSystemsV2 []*prover.ProvingSystemV2
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

	var proof *prover.Proof
	var proofError *Error

	proofRequestMeta, err := prover.ParseProofRequestMeta(buf)
	if err != nil {
		logging.Logger().Info().Msg("error parsing circuit type")
		logging.Logger().Info().Msg(err.Error())
		malformedBodyError(err).send(w)
		return
	}
	logging.Logger().Info().Msgf("proofRequestMeta %+v", proofRequestMeta)

	switch proofRequestMeta.CircuitType {
	case prover.InclusionCircuitType:
		{
			proof, proofError = handler.inclusionProof(buf, proofRequestMeta)
		}
	case prover.NonInclusionCircuitType:
		proof, proofError = handler.nonInclusionProof(buf, proofRequestMeta)
	case prover.CombinedCircuitType:
		proof, proofError = handler.combinedProof(buf, proofRequestMeta)
	case prover.BatchUpdateCircuitType:
		proof, proofError = handler.batchUpdateProof(buf)
	case prover.BatchAppendWithProofsCircuitType:
		proof, proofError = handler.batchAppendWithProofsHandler(buf)
	case prover.BatchAddressAppendCircuitType:
		proof, proofError = handler.batchAddressAppendProof(buf)
	default:
		proofError = malformedBodyError(fmt.Errorf("unknown circuit type"))
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

func (handler proveHandler) batchAddressAppendProof(buf []byte) (*prover.Proof, *Error) {
	var params prover.BatchAddressAppendParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)
	}

	treeHeight := params.TreeHeight
	batchSize := params.BatchSize

	var ps *prover.ProvingSystemV2
	for _, provingSystem := range handler.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchAddressAppendCircuitType && provingSystem.TreeHeight == treeHeight && provingSystem.BatchSize == batchSize {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("batch address append: no proving system for tree height %d and batch size %d", treeHeight, batchSize))
	}

	proof, err := ps.ProveBatchAddressAppend(&params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) batchAppendWithProofsHandler(buf []byte) (*prover.Proof, *Error) {
	var params prover.BatchAppendWithProofsParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("Error during JSON unmarshalling")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)
	}

	treeHeight := params.Height
	batchSize := params.BatchSize

	var ps *prover.ProvingSystemV2
	for _, provingSystem := range handler.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchAppendWithProofsCircuitType && provingSystem.TreeHeight == treeHeight && provingSystem.BatchSize == batchSize {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for tree height %d and batch size %d", treeHeight, batchSize))
	}

	proof, err := ps.ProveBatchAppendWithProofs(&params)
	if err != nil {
		logging.Logger().Err(err).Msg("Error during proof generation")
		return nil, provingError(err)
	}

	return proof, nil
}

func (handler proveHandler) batchUpdateProof(buf []byte) (*prover.Proof, *Error) {
	var params prover.BatchUpdateParameters
	err := json.Unmarshal(buf, &params)
	if err != nil {
		logging.Logger().Info().Msg("error Unmarshal")
		logging.Logger().Info().Msg(err.Error())
		return nil, malformedBodyError(err)
	}

	treeHeight := params.Height
	batchSize := params.BatchSize

	var ps *prover.ProvingSystemV2
	for _, provingSystem := range handler.provingSystemsV2 {
		if provingSystem.CircuitType == prover.BatchUpdateCircuitType && provingSystem.TreeHeight == treeHeight && provingSystem.BatchSize == batchSize {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for tree height %d and batch size %d", treeHeight, batchSize))
	}

	proof, err := ps.ProveBatchUpdate(&params)
	if err != nil {
		logging.Logger().Err(err)
		return nil, provingError(err)
	}
	return proof, nil
}

func (handler proveHandler) inclusionProof(buf []byte, proofRequestMeta prover.ProofRequestMeta) (*prover.Proof, *Error) {

	var ps *prover.ProvingSystemV1
	for _, provingSystem := range handler.provingSystemsV1 {
		if provingSystem.InclusionNumberOfCompressedAccounts == uint32(proofRequestMeta.NumInputs) && provingSystem.InclusionTreeHeight == uint32(proofRequestMeta.StateTreeHeight) && provingSystem.Version == uint32(proofRequestMeta.Version) && provingSystem.NonInclusionNumberOfCompressedAccounts == uint32(0) {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
	fmt.Println(proofRequestMeta)
	if proofRequestMeta.Version == 0 {
		var proof *prover.Proof
		var params prover.LegacyInclusionParameters

		var err = json.Unmarshal(buf, &params)

		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err = ps.LegacyProveInclusion(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else if proofRequestMeta.Version == 1 {
		var proof *prover.Proof
		var params prover.InclusionParameters

		var err = json.Unmarshal(buf, &params)
		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err = ps.ProveInclusion(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else {
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
}

func (handler proveHandler) nonInclusionProof(buf []byte, proofRequestMeta prover.ProofRequestMeta) (*prover.Proof, *Error) {

	var ps *prover.ProvingSystemV1
	for _, provingSystem := range handler.provingSystemsV1 {
		if provingSystem.NonInclusionNumberOfCompressedAccounts == uint32(proofRequestMeta.NumAddresses) && provingSystem.NonInclusionTreeHeight == uint32(proofRequestMeta.AddressTreeHeight) && provingSystem.InclusionNumberOfCompressedAccounts == uint32(0) {
			ps = provingSystem
			break
		}
	}

	if ps == nil {
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}

	if proofRequestMeta.AddressTreeHeight == 26 {
		var proof *prover.Proof
		var params prover.LegacyNonInclusionParameters

		var err = json.Unmarshal(buf, &params)
		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err = ps.LegacyProveNonInclusion(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else if proofRequestMeta.AddressTreeHeight == 40 {
		var proof *prover.Proof
		var params prover.NonInclusionParameters

		var err = json.Unmarshal(buf, &params)
		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)
		}
		proof, err = ps.ProveNonInclusion(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else {
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
}

func (handler proveHandler) combinedProof(buf []byte, proofRequestMeta prover.ProofRequestMeta) (*prover.Proof, *Error) {
	var rawInput map[string]interface{}
	json.Unmarshal(buf, &rawInput)

	var ps *prover.ProvingSystemV1
	for _, provingSystem := range handler.provingSystemsV1 {
		fmt.Printf("provingSystem inputs %+v\n", provingSystem.InclusionNumberOfCompressedAccounts)
		fmt.Printf("provingSystem addresses %+v\n", provingSystem.NonInclusionNumberOfCompressedAccounts)
		fmt.Printf("provingSystem inclusionTreeHeight %+v\n", provingSystem.InclusionTreeHeight)
		fmt.Printf("provingSystem nonInclusionTreeHeight %+v\n", provingSystem.NonInclusionTreeHeight)
		if provingSystem.InclusionNumberOfCompressedAccounts == proofRequestMeta.NumInputs && provingSystem.NonInclusionNumberOfCompressedAccounts == proofRequestMeta.NumAddresses && provingSystem.InclusionTreeHeight == proofRequestMeta.StateTreeHeight && provingSystem.NonInclusionTreeHeight == proofRequestMeta.AddressTreeHeight {
			ps = provingSystem
			break
		}
	}

	if proofRequestMeta.AddressTreeHeight == 26 {
		var proof *prover.Proof
		var params prover.LegacyCombinedParameters

		var err = json.Unmarshal(buf, &params)

		if err != nil {
			logging.Logger().Info().Msg("error Unmarshal")
			logging.Logger().Info().Msg(err.Error())
			return nil, malformedBodyError(err)

		}

		var inclusionNumberOfCompressedAccounts = uint32(len(params.InclusionParameters.Inputs))
		var nonInclusionNumberOfCompressedAccounts = uint32(len(params.NonInclusionParameters.Inputs))

		if ps == nil {
			return nil, provingError(fmt.Errorf("no proving system for %d inclusion compressedAccounts & %d non-inclusion", inclusionNumberOfCompressedAccounts, nonInclusionNumberOfCompressedAccounts))
		}
		proof, err = ps.LegacyProveCombined(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else if proofRequestMeta.AddressTreeHeight == 40 {
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

		if ps == nil {
			return nil, provingError(fmt.Errorf("no proving system for %d inclusion compressedAccounts & %d non-inclusion", inclusionNumberOfCompressedAccounts, nonInclusionNumberOfCompressedAccounts))
		}
		proof, err = ps.ProveCombined(&params)
		if err != nil {
			logging.Logger().Err(err)
			return nil, provingError(err)
		}
		return proof, nil
	} else {
		return nil, provingError(fmt.Errorf("no proving system for %+v proofRequest", proofRequestMeta))
	}
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
