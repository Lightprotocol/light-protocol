package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type BatchUpdateProofInputsJSON struct {
	PreRoot      string     `json:"preRoot"`
	PostRoot     string     `json:"postRoot"`
	StartIndex   uint32     `json:"startIndex"`
	OldLeaves    []string   `json:"oldLeaves"`
	NewLeaves    []string   `json:"newLeaves"`
	MerkleProofs [][]string `json:"merkleProofs"`
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
	paramsJson.Inputs.PreRoot = toHex(&p.PreRoot)
	paramsJson.Inputs.PostRoot = toHex(&p.PostRoot)
	paramsJson.Inputs.StartIndex = p.StartIndex
	paramsJson.Inputs.OldLeaves = make([]string, len(p.OldLeaves))
	paramsJson.Inputs.NewLeaves = make([]string, len(p.NewLeaves))
	for i := 0; i < len(p.OldLeaves); i++ {
		paramsJson.Inputs.OldLeaves[i] = toHex(&p.OldLeaves[i])
		paramsJson.Inputs.NewLeaves[i] = toHex(&p.NewLeaves[i])
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
	err = fromHex(&p.PreRoot, params.Inputs.PreRoot)
	if err != nil {
		return err
	}
	err = fromHex(&p.PostRoot, params.Inputs.PostRoot)
	if err != nil {
		return err
	}
	p.StartIndex = params.Inputs.StartIndex
	p.OldLeaves = make([]big.Int, len(params.Inputs.OldLeaves))
	p.NewLeaves = make([]big.Int, len(params.Inputs.NewLeaves))
	for i := 0; i < len(params.Inputs.OldLeaves); i++ {
		err = fromHex(&p.OldLeaves[i], params.Inputs.OldLeaves[i])
		if err != nil {
			return err
		}
		err = fromHex(&p.NewLeaves[i], params.Inputs.NewLeaves[i])
		if err != nil {
			return err
		}
	}
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
