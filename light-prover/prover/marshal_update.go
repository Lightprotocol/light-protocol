package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type BatchUpdateProofInputsJSON struct {
	PublicInputHash     string     `json:"publicInputHash"`
	OldRoot             string     `json:"oldRoot"`
	NewRoot             string     `json:"newRoot"`
	LeavesHashchainHash string     `json:"leavesHashchainHash"`
	Leaves              []string   `json:"leaves"`
	MerkleProofs        [][]string `json:"newMerkleProofs"`
	PathIndices         []uint32   `json:"pathIndices"`
	Height              uint32     `json:"height"`
	BatchSize           uint32     `json:"batchSize"`
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

func (p *BatchUpdateParameters) CreateBatchUpdateParametersJSON() BatchUpdateProofInputsJSON {
	paramsJson := BatchUpdateProofInputsJSON{}
	paramsJson.PublicInputHash = toHex(p.PublicInputHash)
	paramsJson.OldRoot = toHex(p.OldRoot)
	paramsJson.NewRoot = toHex(p.NewRoot)
	paramsJson.LeavesHashchainHash = toHex(p.LeavesHashchainHash)
	paramsJson.Height = p.Height
	paramsJson.BatchSize = p.BatchSize

	paramsJson.Leaves = make([]string, len(p.Leaves))
	for i := 0; i < len(p.Leaves); i++ {
		paramsJson.Leaves[i] = toHex(p.Leaves[i])
	}

	paramsJson.PathIndices = make([]uint32, len(p.PathIndices))
	for i := 0; i < len(p.PathIndices); i++ {
		paramsJson.PathIndices[i] = p.PathIndices[i]
	}

	paramsJson.MerkleProofs = make([][]string, len(p.MerkleProofs))
	for i := 0; i < len(p.MerkleProofs); i++ {
		paramsJson.MerkleProofs[i] = make([]string, len(p.MerkleProofs[i]))
		for j := 0; j < len(p.MerkleProofs[i]); j++ {
			paramsJson.MerkleProofs[i][j] = toHex(&p.MerkleProofs[i][j])
		}
	}

	return paramsJson
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
	err = fromHex(p.OldRoot, params.OldRoot)
	if err != nil {
		return err
	}

	p.NewRoot = new(big.Int)
	err = fromHex(p.NewRoot, params.NewRoot)
	if err != nil {
		return err
	}

	p.LeavesHashchainHash = new(big.Int)
	err = fromHex(p.LeavesHashchainHash, params.LeavesHashchainHash)
	if err != nil {
		return err
	}

	p.Leaves = make([]*big.Int, len(params.Leaves))
	for i := 0; i < len(params.Leaves); i++ {
		p.Leaves[i] = new(big.Int)
		err = fromHex(p.Leaves[i], params.Leaves[i])
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
			err = fromHex(&p.MerkleProofs[i][j], params.MerkleProofs[i][j])
			if err != nil {
				return err
			}
		}
	}

	p.PublicInputHash = new(big.Int)
	err = fromHex(p.PublicInputHash, params.PublicInputHash)
	if err != nil {
		return err
	}
	return nil
}
