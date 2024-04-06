package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type InclusionParametersJSON struct {
	Roots          []string   `json:"roots"`
	InPathIndices  []uint32   `json:"inPathIndices"`
	InPathElements [][]string `json:"inPathElements"`
	Leaf           []string   `json:"leaves"`
}

func ParseInput(inputJSON string) (InclusionParameters, error) {
	var proofData InclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return InclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *InclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := InclusionParametersJSON{}

	paramsJson.Roots = make([]string, len(p.Roots))
	for i := 0; i < len(p.Roots); i++ {
		paramsJson.Roots[i] = toHex(&p.Roots[i])
	}

	paramsJson.InPathIndices = make([]uint32, len(p.InPathIndices))
	paramsJson.InPathIndices = p.InPathIndices

	paramsJson.InPathElements = make([][]string, len(p.InPathElements))
	for i := 0; i < len(p.InPathElements); i++ {
		paramsJson.InPathElements[i] = make([]string, len(p.InPathElements[i]))
		for j := 0; j < len(p.InPathElements[i]); j++ {
			paramsJson.InPathElements[i][j] = toHex(&p.InPathElements[i][j])
		}
	}

	paramsJson.Leaf = make([]string, len(p.Leaves))
	for i := 0; i < len(p.Leaves); i++ {
		paramsJson.Leaf[i] = toHex(&p.Leaves[i])
	}

	return json.Marshal(paramsJson)
}

func (p *InclusionParameters) UnmarshalJSON(data []byte) error {

	var params InclusionParametersJSON

	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	p.Roots = make([]big.Int, len(params.Roots))
	for i := 0; i < len(params.Roots); i++ {
		err = fromHex(&p.Roots[i], params.Roots[i])
		if err != nil {
			return err
		}
	}

	p.Leaves = make([]big.Int, len(params.Leaf))
	for i := 0; i < len(params.Leaf); i++ {
		err = fromHex(&p.Leaves[i], params.Leaf[i])
		if err != nil {
			return err
		}
	}

	p.InPathIndices = make([]uint32, len(params.InPathIndices))
	p.InPathIndices = params.InPathIndices

	p.InPathElements = make([][]big.Int, len(params.InPathElements))
	for i := 0; i < len(params.InPathElements); i++ {
		p.InPathElements[i] = make([]big.Int, len(params.InPathElements[i]))
		for j := 0; j < len(params.InPathElements[i]); j++ {
			err = fromHex(&p.InPathElements[i][j], params.InPathElements[i][j])
			if err != nil {
				return err
			}
		}
	}

	return nil
}
