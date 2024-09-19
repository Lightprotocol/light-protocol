package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type BatchUpdateProofInputsJSON struct {
	OldRoot             string     `json:"oldRoot"`
	NewRoot             string     `json:"newRoot"`
	LeavesHashchainHash string     `json:"leavesHashchainHash"`
	Leaves              []string   `json:"leaves"`
	MerkleProofs        [][]string `json:"newMerkleProofs"`
	PathIndices         []uint32   `json:"pathIndices"`
	HashChainStartIndex uint32     `json:"hashChainStartIndex"`
	Height              uint32     `json:"height"`
	BatchSize           uint32     `json:"batchSize"`
}
type BatchUpdateParametersJSON struct {
	Inputs BatchUpdateProofInputsJSON `json:"batch-update-inputs"`
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
	paramsJson := p.CreateBatchUpdateParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *BatchUpdateParameters) CreateBatchUpdateParametersJSON() BatchUpdateParametersJSON {
	paramsJson := BatchUpdateParametersJSON{}
	paramsJson.Inputs.OldRoot = toHex(p.OldRoot)
	paramsJson.Inputs.NewRoot = toHex(p.NewRoot)
	paramsJson.Inputs.LeavesHashchainHash = toHex(p.LeavesHashchainHash)
	paramsJson.Inputs.HashChainStartIndex = p.HashChainStartIndex
	paramsJson.Inputs.Height = p.Height
	paramsJson.Inputs.BatchSize = p.BatchSize

	paramsJson.Inputs.Leaves = make([]string, len(p.Leaves))
	for i := 0; i < len(p.Leaves); i++ {
		paramsJson.Inputs.Leaves[i] = toHex(p.Leaves[i])
	}

	paramsJson.Inputs.PathIndices = make([]uint32, len(p.PathIndices))
	for i := 0; i < len(p.PathIndices); i++ {
		paramsJson.Inputs.PathIndices[i] = p.PathIndices[i]
	}

	paramsJson.Inputs.MerkleProofs = make([][]string, len(p.MerkleProofs))
	for i := 0; i < len(p.MerkleProofs); i++ {
		paramsJson.Inputs.MerkleProofs[i] = make([]string, len(p.MerkleProofs[i]))
		for j := 0; j < len(p.MerkleProofs[i]); j++ {
			paramsJson.Inputs.MerkleProofs[i][j] = toHex(&p.MerkleProofs[i][j])
		}
	}

	return paramsJson
}

func (p *BatchUpdateParameters) UnmarshalJSON(data []byte) error {
	var params BatchUpdateParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}
	return p.UpdateWithJSON(params)
}

func (p *BatchUpdateParameters) UpdateWithJSON(params BatchUpdateParametersJSON) error {
	var err error

	p.OldRoot = new(big.Int)
	err = fromHex(p.OldRoot, params.Inputs.OldRoot)
	if err != nil {
		return err
	}

	p.NewRoot = new(big.Int)
	err = fromHex(p.NewRoot, params.Inputs.NewRoot)
	if err != nil {
		return err
	}

	p.LeavesHashchainHash = new(big.Int)
	err = fromHex(p.LeavesHashchainHash, params.Inputs.LeavesHashchainHash)
	if err != nil {
		return err
	}

	p.HashChainStartIndex = params.Inputs.HashChainStartIndex
	p.Height = params.Inputs.Height
	p.BatchSize = params.Inputs.BatchSize

	p.Leaves = make([]*big.Int, len(params.Inputs.Leaves))
	for i := 0; i < len(params.Inputs.Leaves); i++ {
		p.Leaves[i] = new(big.Int)
		err = fromHex(p.Leaves[i], params.Inputs.Leaves[i])
		if err != nil {
			return err
		}
	}

	p.PathIndices = make([]uint32, len(params.Inputs.PathIndices))
	copy(p.PathIndices, params.Inputs.PathIndices)

	p.MerkleProofs = make([][]big.Int, len(params.Inputs.MerkleProofs))
	for i := 0; i < len(params.Inputs.MerkleProofs); i++ {
		p.MerkleProofs[i] = make([]big.Int, len(params.Inputs.MerkleProofs[i]))
		for j := 0; j < len(params.Inputs.MerkleProofs[i]); j++ {
			err = fromHex(&p.MerkleProofs[i][j], params.Inputs.MerkleProofs[i][j])
			if err != nil {
				return err
			}
		}
	}

	return nil
}
