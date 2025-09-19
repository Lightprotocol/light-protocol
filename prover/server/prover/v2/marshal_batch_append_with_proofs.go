package v2

import (
	"encoding/json"
	"light/light-prover/prover/common"
	"math/big"
)

type BatchAppendInputsJSON struct {
	CircuitType         common.CircuitType `json:"circuitType"`
	StateTreeHeight     uint32             `json:"stateTreeHeight"`
	PublicInputHash     string             `json:"publicInputHash"`
	OldRoot             string             `json:"oldRoot"`
	NewRoot             string             `json:"newRoot"`
	LeavesHashchainHash string             `json:"leavesHashchainHash"`
	StartIndex          uint64             `json:"startIndex"`
	OldLeaves           []string           `json:"oldLeaves"`
	Leaves              []string           `json:"leaves"`
	MerkleProofs        [][]string         `json:"merkleProofs"`
	Height              uint32             `json:"height"`
	BatchSize           uint32             `json:"batchSize"`
}

func (p *BatchAppendParameters) MarshalJSON() ([]byte, error) {
	paramsJSON := p.createBatchAppendParametersJSON()
	return json.Marshal(paramsJSON)
}

func (p *BatchAppendParameters) createBatchAppendParametersJSON() BatchAppendInputsJSON {
	paramsJSON := BatchAppendInputsJSON{
		CircuitType:         common.BatchAppendCircuitType,
		PublicInputHash:     common.ToHex(p.PublicInputHash),
		OldRoot:             common.ToHex(p.OldRoot),
		NewRoot:             common.ToHex(p.NewRoot),
		LeavesHashchainHash: common.ToHex(p.LeavesHashchainHash),
		StartIndex:          p.StartIndex,
		Height:              p.Height,
		BatchSize:           p.BatchSize,
	}

	paramsJSON.OldLeaves = make([]string, len(p.OldLeaves))
	paramsJSON.Leaves = make([]string, len(p.Leaves))
	paramsJSON.MerkleProofs = make([][]string, len(p.MerkleProofs))

	for i := 0; i < len(p.Leaves); i++ {
		paramsJSON.OldLeaves[i] = common.ToHex(p.OldLeaves[i])
		paramsJSON.Leaves[i] = common.ToHex(p.Leaves[i])

		paramsJSON.MerkleProofs[i] = make([]string, len(p.MerkleProofs[i]))
		for j := 0; j < len(p.MerkleProofs[i]); j++ {
			paramsJSON.MerkleProofs[i][j] = common.ToHex(&p.MerkleProofs[i][j])
		}
	}

	return paramsJSON
}

func (p *BatchAppendParameters) UnmarshalJSON(data []byte) error {
	var params BatchAppendInputsJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	return p.updateWithJSON(params)
}

func (p *BatchAppendParameters) updateWithJSON(params BatchAppendInputsJSON) error {
	var err error

	p.StartIndex = params.StartIndex
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

	p.OldLeaves = make([]*big.Int, len(params.OldLeaves))
	p.Leaves = make([]*big.Int, len(params.Leaves))
	for i := 0; i < len(params.Leaves); i++ {
		p.OldLeaves[i] = new(big.Int)
		err = common.FromHex(p.OldLeaves[i], params.OldLeaves[i])
		if err != nil {
			return err
		}
		p.Leaves[i] = new(big.Int)
		err = common.FromHex(p.Leaves[i], params.Leaves[i])
		if err != nil {
			return err
		}
	}

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
