package v2

import (
	"encoding/json"
	"fmt"
	"light/light-prover/prover/common"
	"math/big"
)

type BatchUpdateProofInputsJSON struct {
	CircuitType         common.CircuitType `json:"circuitType"`
	StateTreeHeight     uint32             `json:"stateTreeHeight"`
	PublicInputHash     string             `json:"publicInputHash"`
	OldRoot             string             `json:"oldRoot"`
	NewRoot             string             `json:"newRoot"`
	TxHashes            []string           `json:"txHashes"`
	LeavesHashchainHash string             `json:"leavesHashchainHash"`
	Leaves              []string           `json:"leaves"`
	OldLeaves           []string           `json:"oldLeaves"`
	MerkleProofs        [][]string         `json:"newMerkleProofs"`
	PathIndices         []uint32           `json:"pathIndices"`
	Height              uint32             `json:"height"`
	BatchSize           uint32             `json:"batchSize"`
}

func ParseBatchUpdateInput(inputJSON string) (BatchUpdateParameters, error) {
	var proofData BatchUpdateParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return BatchUpdateParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *BatchUpdateParameters) MarshalJSON() ([]byte, error) {
	paramsJson, err := p.CreateBatchUpdateParametersJSON()
	if err != nil {
		return nil, err
	}
	return json.Marshal(paramsJson)
}

func (p *BatchUpdateParameters) CreateBatchUpdateParametersJSON() (BatchUpdateProofInputsJSON, error) {
	paramsJson := BatchUpdateProofInputsJSON{}
	paramsJson.CircuitType = common.BatchUpdateCircuitType
	paramsJson.StateTreeHeight = uint32(len(p.MerkleProofs[0]))
	paramsJson.PublicInputHash = common.ToHex(p.PublicInputHash)
	paramsJson.OldRoot = common.ToHex(p.OldRoot)
	paramsJson.NewRoot = common.ToHex(p.NewRoot)
	paramsJson.LeavesHashchainHash = common.ToHex(p.LeavesHashchainHash)
	paramsJson.Height = p.Height
	paramsJson.BatchSize = p.BatchSize

	// Validate that all slices have the same length
	expectedLen := len(p.Leaves)
	if len(p.TxHashes) != expectedLen || len(p.PathIndices) != expectedLen ||
		len(p.MerkleProofs) != expectedLen || len(p.OldLeaves) != expectedLen {
		return BatchUpdateProofInputsJSON{}, fmt.Errorf("inconsistent slice lengths: leaves=%d, txHashes=%d, pathIndices=%d, merkleProofs=%d, oldLeaves=%d",
			len(p.Leaves), len(p.TxHashes), len(p.PathIndices), len(p.MerkleProofs), len(p.OldLeaves))
	}

	paramsJson.TxHashes = make([]string, len(p.TxHashes))
	paramsJson.Leaves = make([]string, len(p.Leaves))
	paramsJson.PathIndices = make([]uint32, len(p.PathIndices))
	paramsJson.MerkleProofs = make([][]string, len(p.MerkleProofs))
	paramsJson.OldLeaves = make([]string, len(p.OldLeaves))
	for i := 0; i < len(p.Leaves); i++ {
		paramsJson.OldLeaves[i] = common.ToHex(p.OldLeaves[i])
		paramsJson.Leaves[i] = common.ToHex(p.Leaves[i])
		paramsJson.TxHashes[i] = common.ToHex(p.TxHashes[i])

		paramsJson.PathIndices[i] = p.PathIndices[i]

		paramsJson.MerkleProofs[i] = make([]string, len(p.MerkleProofs[i]))
		for j := 0; j < len(p.MerkleProofs[i]); j++ {
			paramsJson.MerkleProofs[i][j] = common.ToHex(&p.MerkleProofs[i][j])
		}
	}

	return paramsJson, nil
}

func (p *BatchUpdateParameters) UnmarshalJSON(data []byte) error {
	var params BatchUpdateProofInputsJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	return p.UpdateWithJSON(params)
}

func (p *BatchUpdateParameters) UpdateWithJSON(params BatchUpdateProofInputsJSON) error {
	var err error
	p.Height = params.Height
	p.BatchSize = params.BatchSize

	p.OldRoot = new(big.Int)
	err = common.FromHex(p.OldRoot, params.OldRoot)
	if err != nil {
		return err
	}

	p.NewRoot = new(big.Int)
	err = common.FromHex(p.NewRoot, params.NewRoot)
	if err != nil {
		return err
	}

	p.LeavesHashchainHash = new(big.Int)
	err = common.FromHex(p.LeavesHashchainHash, params.LeavesHashchainHash)
	if err != nil {
		return err
	}

	p.TxHashes = make([]*big.Int, len(params.TxHashes))
	p.Leaves = make([]*big.Int, len(params.Leaves))
	p.OldLeaves = make([]*big.Int, len(params.OldLeaves))
	for i := 0; i < len(params.Leaves); i++ {
		p.Leaves[i] = new(big.Int)
		err = common.FromHex(p.Leaves[i], params.Leaves[i])
		if err != nil {
			return err
		}
		p.TxHashes[i] = new(big.Int)
		err = common.FromHex(p.TxHashes[i], params.TxHashes[i])
		if err != nil {
			return err
		}
		p.OldLeaves[i] = new(big.Int)
		err = common.FromHex(p.OldLeaves[i], params.OldLeaves[i])
		if err != nil {
			return err
		}
	}

	p.PathIndices = make([]uint32, len(params.PathIndices))
	copy(p.PathIndices, params.PathIndices)

	p.MerkleProofs = make([][]big.Int, len(params.MerkleProofs))
	for i := 0; i < len(params.MerkleProofs); i++ {
		p.MerkleProofs[i] = make([]big.Int, len(params.MerkleProofs[i]))
		for j := 0; j < len(params.MerkleProofs[i]); j++ {
			err = common.FromHex(&p.MerkleProofs[i][j], params.MerkleProofs[i][j])
			if err != nil {
				return err
			}
		}
	}

	p.PublicInputHash = new(big.Int)
	err = common.FromHex(p.PublicInputHash, params.PublicInputHash)
	if err != nil {
		return err
	}
	return nil
}
