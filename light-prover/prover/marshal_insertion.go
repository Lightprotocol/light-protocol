package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type InsertionProofInputsJSON struct {
	PreRoot      string     `json:"preRoot"`
	PostRoot     string     `json:"postRoot"`
	StartIndex   uint32     `json:"startIndex"`
	Leaves       []string   `json:"leaves"`
	MerkleProofs [][]string `json:"merkleProofs"`
}

type InsertionParametersJSON struct {
	Inputs InsertionProofInputsJSON `json:"insertion-inputs"`
}

func ParseInsertionInput(inputJSON string) (InsertionParameters, error) {
	var proofData InsertionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return InsertionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *InsertionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateInsertionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *InsertionParameters) CreateInsertionParametersJSON() InsertionParametersJSON {
	paramsJson := InsertionParametersJSON{}
	paramsJson.Inputs.PreRoot = toHex(&p.PreRoot)
	paramsJson.Inputs.PostRoot = toHex(&p.PostRoot)
	paramsJson.Inputs.StartIndex = p.StartIndex
	paramsJson.Inputs.Leaves = make([]string, len(p.Leaves))
	for i := 0; i < len(p.Leaves); i++ {
		paramsJson.Inputs.Leaves[i] = toHex(&p.Leaves[i])
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

func (p *InsertionParameters) UnmarshalJSON(data []byte) error {
	var params InsertionParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}
	err = p.UpdateWithJSON(params, err)
	if err != nil {
		return err
	}
	return nil
}

func (p *InsertionParameters) UpdateWithJSON(params InsertionParametersJSON, err error) error {
	err = fromHex(&p.PreRoot, params.Inputs.PreRoot)
	if err != nil {
		return err
	}
	err = fromHex(&p.PostRoot, params.Inputs.PostRoot)
	if err != nil {
		return err
	}
	p.StartIndex = params.Inputs.StartIndex
	p.Leaves = make([]big.Int, len(params.Inputs.Leaves))
	for i := 0; i < len(params.Inputs.Leaves); i++ {
		err = fromHex(&p.Leaves[i], params.Inputs.Leaves[i])
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
