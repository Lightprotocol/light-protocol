package masp

import (
	"encoding/json"
	"fmt"

	"light/light-prover/prover/common"
)

// RootContext binds a MASP proof request to a specific point in the public
// state/nullifier root history. The on-chain verifier uses these to look up
// roots and to expire stale jobs.
type RootContext struct {
	UtxoTreeID         string `json:"utxoTreeId"`
	UtxoRoot           string `json:"utxoRoot"`
	UtxoRootIndex      uint64 `json:"utxoRootIndex"`
	UtxoRootSequence   uint64 `json:"utxoRootSequence"`
	NullifierTreeID    string `json:"nullifierTreeId"`
	NullifierRoot      string `json:"nullifierRoot"`
	NullifierRootIndex uint64 `json:"nullifierRootIndex"`
	NullifierRootSeq   uint64 `json:"nullifierRootSequence"`
	// ExpirySlot tells the prover/relayer when to discard stale work.
	ExpirySlot uint64 `json:"expirySlot,omitempty"`
}

// EncryptedWitnessPayload carries a witness encrypted to a specific TEE
// enclave's attested public key. Used by the production server-side proving
// path so plaintext witnesses never sit on the prover-server host.
type EncryptedWitnessPayload struct {
	EnclavePubkey string `json:"enclavePubkey"`
	Ciphertext    string `json:"ciphertext"`
	Nonce         string `json:"nonce"`
	AuthTag       string `json:"authTag"`
}

// SignedUserIntent is a signature over the operation commitment / public
// inputs hash, expiry, owner nonce, and the requested transition. The prover
// will not accept a request that lacks a valid signature for any flow that
// uses spend-scoped material such as `domain_nullifier_secret_i`.
type SignedUserIntent struct {
	OwnerPubkey     string `json:"ownerPubkey"`
	OperationCommit string `json:"operationCommitment"`
	Expiry          uint64 `json:"expiry"`
	OwnerNonce      uint64 `json:"ownerNonce"`
	TransitionTag   string `json:"transitionTag"`
	Signature       string `json:"signature"`
}

// MaspBaseRequest holds the fields shared by every MASP proof request.
type MaspBaseRequest struct {
	CircuitType      common.CircuitType       `json:"circuitType"`
	NInputs          uint32                   `json:"nInputs"`
	NOutputs         uint32                   `json:"nOutputs"`
	RootContext      RootContext              `json:"rootContext"`
	OperationCommit  string                   `json:"operationCommitment,omitempty"`
	PublicInputsHash string                   `json:"publicInputsHash,omitempty"`
	UserIntent       *SignedUserIntent        `json:"userIntent,omitempty"`
	EncryptedWitness *EncryptedWitnessPayload `json:"encryptedWitness,omitempty"`
}

// MaspUtxoProofRequest produces the UtxoCircuit proof: nullifiers from
// in_commit + leaf_index + dns; per-output commitment recomputation; value
// conservation; P-256 spend authorization; and PublicInputsHash binding.
type MaspUtxoProofRequest struct {
	MaspBaseRequest
	// LocalWitness is set by dev/test clients that pass plaintext witnesses
	// directly. Production deployments must use EncryptedWitness instead.
	LocalWitness *MaspUtxoLocalWitness `json:"localWitness,omitempty"`
}

// MaspTreeProofRequest produces the TreeCircuit proof: state inclusion +
// indexed nullifier non-inclusion + nullifier reconstruction.
type MaspTreeProofRequest struct {
	MaspBaseRequest
	LocalWitness *MaspTreeLocalWitness `json:"localWitness,omitempty"`
}

// MaspProofBundleRequest asks the prover to return both a TreeProof and a
// UtxoProof for the same nullifier set. The two proofs share NullifierChain
// so the on-chain verifier reconstructs one digest and feeds it into both.
type MaspProofBundleRequest struct {
	MaspBaseRequest
	LocalWitness *MaspBundleLocalWitness `json:"localWitness,omitempty"`
}

// MaspUtxoLocalWitness mirrors UtxoCircuit's private witness fields as
// hex-encoded strings. Field shapes track NInputs/NOutputs.
type MaspUtxoLocalWitness struct {
	InOwner     []string `json:"inOwner"`
	InSpl       []string `json:"inSpl"`
	InSol       []string `json:"inSol"`
	InBlinding  []string `json:"inBlinding"`
	InDataHash  []string `json:"inDataHash"`
	InSeed      []string `json:"inSeed"`
	InProgramID []string `json:"inProgramId"`
	InLeafIndex []string `json:"inLeafIndex"`

	NullifierSecret string `json:"nullifierSecret"`

	OutOwner             []string `json:"outOwner"`
	OutSpl               []string `json:"outSpl"`
	OutSol               []string `json:"outSol"`
	OutBlinding          []string `json:"outBlinding"`
	OutDataHash          []string `json:"outDataHash"`
	OutOwnerIsProgram    []string `json:"outOwnerIsProgram"`
	OutOwnerProgramIndex []string `json:"outOwnerProgramIndex"`
	OutSeed              []string `json:"outSeed"`

	TxBlinding string `json:"txBlinding"`

	PubX [2]string `json:"pubX"`
	PubY [2]string `json:"pubY"`
	SigR [2]string `json:"sigR"`
	SigS [2]string `json:"sigS"`

	Nullifiers         []string `json:"nullifiers"`
	OutputCommitments  []string `json:"outputCommitments"`
	TxHash             string   `json:"txHash"`
	SeedsHashchain     string   `json:"seedsHashchain"`
	ProgramIDHashchain string   `json:"programIdHashchain"`
	ShaTxHash          string   `json:"shaTxHash"`
	NullifierChain     string   `json:"nullifierChain"`
}

// MaspTreeLocalWitness mirrors TreeCircuit's private witness fields.
type MaspTreeLocalWitness struct {
	InCommit             []string   `json:"inCommit"`
	AccountOwnerHash     []string   `json:"accountOwnerHash"`
	AccountTreeHash      []string   `json:"accountTreeHash"`
	AccountDiscriminator []string   `json:"accountDiscriminator"`
	StatePath            [][]string `json:"statePath"`
	StateDirs            [][]string `json:"stateDirs"`
	DomainDNS            []string   `json:"domainDns"`

	NfLowValue  []string   `json:"nfLowValue"`
	NfNextValue []string   `json:"nfNextValue"`
	NfLowPath   [][]string `json:"nfLowPath"`
	NfLowDirs   [][]string `json:"nfLowDirs"`

	StateRoots     []string `json:"stateRoots"`
	NullifierRoots []string `json:"nullifierRoots"`
	Nullifiers     []string `json:"nullifiers"`
}

// MaspBundleLocalWitness wraps both witnesses for a bundled request.
type MaspBundleLocalWitness struct {
	Utxo MaspUtxoLocalWitness `json:"utxo"`
	Tree MaspTreeLocalWitness `json:"tree"`
}

// ParseMaspBaseRequest decodes the shared fields of a MASP request and runs
// the minimum validity checks (circuit type, shape sanity).
func ParseMaspBaseRequest(buf []byte) (*MaspBaseRequest, error) {
	var req MaspBaseRequest
	if err := json.Unmarshal(buf, &req); err != nil {
		return nil, fmt.Errorf("decode MASP request: %w", err)
	}
	if !common.IsMaspCircuit(req.CircuitType) {
		return nil, fmt.Errorf("not a MASP circuit type: %q", req.CircuitType)
	}
	if req.NInputs == 0 {
		return nil, fmt.Errorf("nInputs must be > 0")
	}
	if req.CircuitType == common.MaspUtxoCircuitType || req.CircuitType == common.MaspBundleCircuitType {
		if req.NOutputs == 0 {
			return nil, fmt.Errorf("nOutputs must be > 0 for %s", req.CircuitType)
		}
	}
	return &req, nil
}
