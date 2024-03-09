package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type InclusionParametersJSON struct {
	Root           []string   `json:"root"`
	InPathIndices  []uint32   `json:"inPathIndices"`
	InPathElements [][]string `json:"inPathElements"`
	Leaf           []string   `json:"leaf"`
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

	paramsJson.Root = make([]string, len(p.Root))
	for i := 0; i < len(p.Root); i++ {
		paramsJson.Root[i] = toHex(&p.Root[i])
	}

	paramsJson.InPathIndices = make([]uint32, len(p.InPathIndices))
	for i := 0; i < len(p.InPathIndices); i++ {
		paramsJson.InPathIndices[i] = p.InPathIndices[i]
	}

	paramsJson.InPathElements = make([][]string, len(p.InPathElements))
	for i := 0; i < len(p.InPathElements); i++ {
		paramsJson.InPathElements[i] = make([]string, len(p.InPathElements[i]))
		for j := 0; j < len(p.InPathElements[i]); j++ {
			paramsJson.InPathElements[i][j] = toHex(&p.InPathElements[i][j])
		}
	}

	paramsJson.Leaf = make([]string, len(p.Leaf))
	for i := 0; i < len(p.Leaf); i++ {
		paramsJson.Leaf[i] = toHex(&p.Leaf[i])
	}

	return json.Marshal(paramsJson)
}

func (p *InclusionParameters) UnmarshalJSON(data []byte) error {

	var params InclusionParametersJSON

	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	p.Root = make([]big.Int, len(params.Root))
	for i := 0; i < len(params.Root); i++ {
		err = fromHex(&p.Root[i], params.Root[i])
		if err != nil {
			return err
		}
	}

	p.Leaf = make([]big.Int, len(params.Leaf))
	for i := 0; i < len(params.Leaf); i++ {
		err = fromHex(&p.Leaf[i], params.Leaf[i])
		if err != nil {
			return err
		}
	}

	p.InPathIndices = make([]uint32, len(params.InPathIndices))
	for i := 0; i < len(params.InPathIndices); i++ {
		p.InPathIndices[i] = params.InPathIndices[i]
	}

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
