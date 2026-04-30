package masp

import (
	"encoding/json"
	"fmt"
	"math/big"
	"strings"

	"light/light-prover/prover/common"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/math/emulated"
	gnarkecdsa "github.com/consensys/gnark/std/signature/ecdsa"
)

// ParseMaspUtxoProofRequest decodes and validates a local/dev MASP UTXO proof
// request. Production server-side proving must use EncryptedWitness inside a
// TEE worker, but this local witness path is needed for end-to-end PoC wiring.
func ParseMaspUtxoProofRequest(buf []byte) (*MaspUtxoProofRequest, error) {
	base, err := ParseMaspBaseRequest(buf)
	if err != nil {
		return nil, err
	}
	if base.CircuitType != common.MaspUtxoCircuitType {
		return nil, fmt.Errorf("expected %s, got %s", common.MaspUtxoCircuitType, base.CircuitType)
	}

	var req MaspUtxoProofRequest
	if err := json.Unmarshal(buf, &req); err != nil {
		return nil, fmt.Errorf("decode MASP UTXO request: %w", err)
	}
	if req.LocalWitness == nil {
		return nil, fmt.Errorf("masp-utxo local/dev proving requires localWitness")
	}
	if req.PublicInputsHash == "" {
		return nil, fmt.Errorf("masp-utxo requires publicInputsHash")
	}
	return &req, nil
}

// ParseMaspTreeProofRequest decodes and validates a local/dev MASP Tree proof
// request.
func ParseMaspTreeProofRequest(buf []byte) (*MaspTreeProofRequest, error) {
	base, err := ParseMaspBaseRequest(buf)
	if err != nil {
		return nil, err
	}
	if base.CircuitType != common.MaspTreeCircuitType {
		return nil, fmt.Errorf("expected %s, got %s", common.MaspTreeCircuitType, base.CircuitType)
	}

	var req MaspTreeProofRequest
	if err := json.Unmarshal(buf, &req); err != nil {
		return nil, fmt.Errorf("decode MASP Tree request: %w", err)
	}
	if req.LocalWitness == nil {
		return nil, fmt.Errorf("masp-tree local/dev proving requires localWitness")
	}
	if req.PublicInputsHash == "" {
		return nil, fmt.Errorf("masp-tree requires publicInputsHash")
	}
	return &req, nil
}

func ProveUtxoRequest(keyManager *common.LazyKeyManager, req *MaspUtxoProofRequest) (*common.Proof, error) {
	ps, err := keyManager.GetMaspSystem(req.CircuitType, req.NInputs, req.NOutputs)
	if err != nil {
		return nil, err
	}
	assignment, err := UtxoAssignmentFromLocalWitness(req.NInputs, req.NOutputs, req.PublicInputsHash, req.LocalWitness)
	if err != nil {
		return nil, err
	}
	return ProveUtxoAssignment(ps, assignment)
}

func ProveTreeRequest(keyManager *common.LazyKeyManager, req *MaspTreeProofRequest) (*common.Proof, error) {
	ps, err := keyManager.GetMaspSystem(req.CircuitType, req.NInputs, req.NOutputs)
	if err != nil {
		return nil, err
	}
	assignment, err := TreeAssignmentFromLocalWitness(req.NInputs, req.PublicInputsHash, req.LocalWitness)
	if err != nil {
		return nil, err
	}
	return ProveTreeAssignment(ps, assignment)
}

func ProveUtxoWitness(ps *common.MaspProofSystem, witness *Witness) (*common.Proof, error) {
	if witness == nil {
		return nil, fmt.Errorf("masp utxo witness is nil")
	}
	return ProveUtxoAssignment(ps, witness.UtxoAssignment())
}

func ProveTreeWitness(ps *common.MaspProofSystem, witness *Witness) (*common.Proof, error) {
	if witness == nil {
		return nil, fmt.Errorf("masp tree witness is nil")
	}
	return ProveTreeAssignment(ps, witness.TreeAssignment())
}

func ProveUtxoAssignment(ps *common.MaspProofSystem, assignment *UtxoCircuit) (*common.Proof, error) {
	if ps == nil {
		return nil, fmt.Errorf("masp utxo proof system is nil")
	}
	if assignment == nil {
		return nil, fmt.Errorf("masp utxo assignment is nil")
	}
	if ps.CircuitType != common.MaspUtxoCircuitType {
		return nil, fmt.Errorf("expected %s proof system, got %s", common.MaspUtxoCircuitType, ps.CircuitType)
	}
	if uint32(assignment.NInputs) != ps.NInputs || uint32(assignment.NOutputs) != ps.NOutputs {
		return nil, fmt.Errorf("masp utxo assignment shape %d/%d does not match proof system %d/%d", assignment.NInputs, assignment.NOutputs, ps.NInputs, ps.NOutputs)
	}

	witness, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}
	return &common.Proof{Proof: proof}, nil
}

func ProveTreeAssignment(ps *common.MaspProofSystem, assignment *TreeCircuit) (*common.Proof, error) {
	if ps == nil {
		return nil, fmt.Errorf("masp tree proof system is nil")
	}
	if assignment == nil {
		return nil, fmt.Errorf("masp tree assignment is nil")
	}
	if ps.CircuitType != common.MaspTreeCircuitType {
		return nil, fmt.Errorf("expected %s proof system, got %s", common.MaspTreeCircuitType, ps.CircuitType)
	}
	if uint32(assignment.NInputs) != ps.NInputs {
		return nil, fmt.Errorf("masp tree assignment shape %d does not match proof system %d", assignment.NInputs, ps.NInputs)
	}

	witness, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		return nil, err
	}
	proof, err := groth16.Prove(ps.ConstraintSystem, ps.ProvingKey, witness)
	if err != nil {
		return nil, err
	}
	return &common.Proof{Proof: proof}, nil
}

func UtxoAssignmentFromLocalWitness(nInputs, nOutputs uint32, publicInputsHash string, local *MaspUtxoLocalWitness) (*UtxoCircuit, error) {
	if local == nil {
		return nil, fmt.Errorf("masp utxo local witness is nil")
	}
	c := NewUtxoCircuit(int(nInputs), int(nOutputs))
	if err := setVector(c.InOwner, local.InOwner, "localWitness.inOwner"); err != nil {
		return nil, err
	}
	if err := setVector(c.InSpl, local.InSpl, "localWitness.inSpl"); err != nil {
		return nil, err
	}
	if err := setVector(c.InSol, local.InSol, "localWitness.inSol"); err != nil {
		return nil, err
	}
	if err := setVector(c.InBlinding, local.InBlinding, "localWitness.inBlinding"); err != nil {
		return nil, err
	}
	if err := setVector(c.InDataHash, local.InDataHash, "localWitness.inDataHash"); err != nil {
		return nil, err
	}
	if err := setVector(c.InSeed, local.InSeed, "localWitness.inSeed"); err != nil {
		return nil, err
	}
	if err := setVector(c.InProgramID, local.InProgramID, "localWitness.inProgramId"); err != nil {
		return nil, err
	}
	if err := setVector(c.InLeafIndex, local.InLeafIndex, "localWitness.inLeafIndex"); err != nil {
		return nil, err
	}
	nullifierSecret, err := parseBigString(local.NullifierSecret, "localWitness.nullifierSecret")
	if err != nil {
		return nil, err
	}
	c.NullifierSecret = nullifierSecret

	if err := setVector(c.OutOwner, local.OutOwner, "localWitness.outOwner"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutSpl, local.OutSpl, "localWitness.outSpl"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutSol, local.OutSol, "localWitness.outSol"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutBlinding, local.OutBlinding, "localWitness.outBlinding"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutDataHash, local.OutDataHash, "localWitness.outDataHash"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutOwnerIsProgram, local.OutOwnerIsProgram, "localWitness.outOwnerIsProgram"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutOwnerProgramIndex, local.OutOwnerProgramIndex, "localWitness.outOwnerProgramIndex"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutSeed, local.OutSeed, "localWitness.outSeed"); err != nil {
		return nil, err
	}

	if err := setVector(c.Nullifiers, local.Nullifiers, "localWitness.nullifiers"); err != nil {
		return nil, err
	}
	if err := setVector(c.OutputCommitments, local.OutputCommitments, "localWitness.outputCommitments"); err != nil {
		return nil, err
	}
	if c.TxHash, err = parseBigString(local.TxHash, "localWitness.txHash"); err != nil {
		return nil, err
	}
	if c.SeedsHashchain, err = parseBigString(local.SeedsHashchain, "localWitness.seedsHashchain"); err != nil {
		return nil, err
	}
	if c.ProgramIDHashchain, err = parseBigString(local.ProgramIDHashchain, "localWitness.programIdHashchain"); err != nil {
		return nil, err
	}
	if c.ShaTxHash, err = parseBigString(local.ShaTxHash, "localWitness.shaTxHash"); err != nil {
		return nil, err
	}
	if c.NullifierChain, err = parseBigString(local.NullifierChain, "localWitness.nullifierChain"); err != nil {
		return nil, err
	}
	if c.PublicInputsHash, err = parseBigString(publicInputsHash, "publicInputsHash"); err != nil {
		return nil, err
	}
	if c.TxBlinding, err = parseBigString(local.TxBlinding, "localWitness.txBlinding"); err != nil {
		return nil, err
	}

	pubX, err := parseSplit256(local.PubX, "localWitness.pubX")
	if err != nil {
		return nil, err
	}
	pubY, err := parseSplit256(local.PubY, "localWitness.pubY")
	if err != nil {
		return nil, err
	}
	sigR, err := parseSplit256(local.SigR, "localWitness.sigR")
	if err != nil {
		return nil, err
	}
	sigS, err := parseSplit256(local.SigS, "localWitness.sigS")
	if err != nil {
		return nil, err
	}
	c.Pub = gnarkecdsa.PublicKey[emulated.P256Fp, emulated.P256Fr]{
		X: emulated.ValueOf[emulated.P256Fp](pubX),
		Y: emulated.ValueOf[emulated.P256Fp](pubY),
	}
	c.Sig = gnarkecdsa.Signature[emulated.P256Fr]{
		R: emulated.ValueOf[emulated.P256Fr](sigR),
		S: emulated.ValueOf[emulated.P256Fr](sigS),
	}
	return c, nil
}

func TreeAssignmentFromLocalWitness(nInputs uint32, publicInputsHash string, local *MaspTreeLocalWitness) (*TreeCircuit, error) {
	if local == nil {
		return nil, fmt.Errorf("masp tree local witness is nil")
	}
	c := NewTreeCircuit(int(nInputs))
	if err := setVector(c.InCommit, local.InCommit, "localWitness.inCommit"); err != nil {
		return nil, err
	}
	if err := setVector(c.AccountOwnerHash, local.AccountOwnerHash, "localWitness.accountOwnerHash"); err != nil {
		return nil, err
	}
	if err := setVector(c.AccountTreeHash, local.AccountTreeHash, "localWitness.accountTreeHash"); err != nil {
		return nil, err
	}
	if err := setVector(c.AccountDiscriminator, local.AccountDiscriminator, "localWitness.accountDiscriminator"); err != nil {
		return nil, err
	}
	if err := setVector(c.DomainDNS, local.DomainDNS, "localWitness.domainDns"); err != nil {
		return nil, err
	}
	if err := setVector(c.NfLowValue, local.NfLowValue, "localWitness.nfLowValue"); err != nil {
		return nil, err
	}
	if err := setVector(c.NfNextValue, local.NfNextValue, "localWitness.nfNextValue"); err != nil {
		return nil, err
	}
	if err := setVector(c.StateRoots, local.StateRoots, "localWitness.stateRoots"); err != nil {
		return nil, err
	}
	if err := setVector(c.NullifierRoots, local.NullifierRoots, "localWitness.nullifierRoots"); err != nil {
		return nil, err
	}
	if err := setVector(c.Nullifiers, local.Nullifiers, "localWitness.nullifiers"); err != nil {
		return nil, err
	}
	if err := setMatrix(c.StatePath, local.StatePath, StateTreeHeight, "localWitness.statePath"); err != nil {
		return nil, err
	}
	if err := setMatrix(c.StateDirs, local.StateDirs, StateTreeHeight, "localWitness.stateDirs"); err != nil {
		return nil, err
	}
	if err := setMatrix(c.NfLowPath, local.NfLowPath, IndexedTreeHeight, "localWitness.nfLowPath"); err != nil {
		return nil, err
	}
	if err := setMatrix(c.NfLowDirs, local.NfLowDirs, IndexedTreeHeight, "localWitness.nfLowDirs"); err != nil {
		return nil, err
	}

	publicHash, err := parseBigString(publicInputsHash, "publicInputsHash")
	if err != nil {
		return nil, err
	}
	c.PublicInputsHash = publicHash
	return c, nil
}

func setVector(dst []frontend.Variable, src []string, field string) error {
	if len(src) != len(dst) {
		return fmt.Errorf("%s length mismatch: got %d, want %d", field, len(src), len(dst))
	}
	for i := range src {
		value, err := parseBigString(src[i], fmt.Sprintf("%s[%d]", field, i))
		if err != nil {
			return err
		}
		dst[i] = value
	}
	return nil
}

func setMatrix(dst [][]frontend.Variable, src [][]string, rowLen int, field string) error {
	if len(src) != len(dst) {
		return fmt.Errorf("%s row count mismatch: got %d, want %d", field, len(src), len(dst))
	}
	for i := range src {
		if len(src[i]) != rowLen {
			return fmt.Errorf("%s[%d] length mismatch: got %d, want %d", field, i, len(src[i]), rowLen)
		}
		if err := setVector(dst[i], src[i], fmt.Sprintf("%s[%d]", field, i)); err != nil {
			return err
		}
	}
	return nil
}

func parseSplit256(parts [2]string, field string) (*big.Int, error) {
	hi, err := parseBigString(parts[0], field+"[0]")
	if err != nil {
		return nil, err
	}
	lo, err := parseBigString(parts[1], field+"[1]")
	if err != nil {
		return nil, err
	}
	if hi.BitLen() > 128 || lo.BitLen() > 128 {
		return nil, fmt.Errorf("%s limbs must be <= 128 bits", field)
	}
	return new(big.Int).Add(new(big.Int).Lsh(hi, 128), lo), nil
}

func parseBigString(input string, field string) (*big.Int, error) {
	if input == "" {
		return nil, fmt.Errorf("%s is required", field)
	}
	base := 10
	value := strings.TrimSpace(input)
	if strings.HasPrefix(value, "0x") || strings.HasPrefix(value, "0X") {
		base = 16
		value = value[2:]
	}
	out, ok := new(big.Int).SetString(value, base)
	if !ok {
		return nil, fmt.Errorf("%s is not a valid integer", field)
	}
	if out.Sign() < 0 {
		return nil, fmt.Errorf("%s must be non-negative", field)
	}
	return out, nil
}
