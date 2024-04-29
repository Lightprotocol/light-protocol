package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type NonInclusionParametersJSON struct {
	Roots          []string   `json:"roots"`
	Values         []string   `json:"values"`
	InPathIndices  []uint32   `json:"inPathIndices"`
	InPathElements [][]string `json:"inPathElements"`

	LeafLowerRangeValues  []string `json:"leafLowerRangeValues"`
	LeafHigherRangeValues []string `json:"leafHigherRangeValues"`
	LeafIndices           []uint32 `json:"leafIndices"`
}

func ParseNonInclusion(inputJSON string) (NonInclusionParameters, error) {
	var proofData NonInclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return NonInclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *NonInclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := NonInclusionParametersJSON{}

	paramsJson.Roots = make([]string, len(p.Roots))
	for i := 0; i < len(p.Roots); i++ {
		paramsJson.Roots[i] = toHex(&p.Roots[i])
	}

	paramsJson.Values = make([]string, len(p.Values))
	for i := 0; i < len(p.Values); i++ {
		paramsJson.Values[i] = toHex(&p.Values[i])
	}

	paramsJson.LeafLowerRangeValues = make([]string, len(p.LeafLowerRangeValues))
	for i := 0; i < len(p.LeafLowerRangeValues); i++ {
		paramsJson.LeafLowerRangeValues[i] = toHex(&p.LeafLowerRangeValues[i])
	}

	paramsJson.LeafHigherRangeValues = make([]string, len(p.LeafHigherRangeValues))
	for i := 0; i < len(p.LeafHigherRangeValues); i++ {
		paramsJson.LeafHigherRangeValues[i] = toHex(&p.LeafHigherRangeValues[i])
	}

	paramsJson.LeafIndices = make([]uint32, len(p.LeafIndices))
	for i := 0; i < len(p.LeafIndices); i++ {
		paramsJson.LeafIndices[i] = p.LeafIndices[i]
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

	return json.Marshal(paramsJson)
}

func (p *NonInclusionParameters) UnmarshalJSON(data []byte) error {

	var params NonInclusionParametersJSON

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

	p.Values = make([]big.Int, len(params.Values))
	for i := 0; i < len(params.Values); i++ {
		err = fromHex(&p.Values[i], params.Values[i])
		if err != nil {
			return err
		}
	}

	p.LeafLowerRangeValues = make([]big.Int, len(params.LeafLowerRangeValues))
	for i := 0; i < len(params.LeafLowerRangeValues); i++ {
		err = fromHex(&p.LeafLowerRangeValues[i], params.LeafLowerRangeValues[i])
		if err != nil {
			return err
		}
	}

	p.LeafHigherRangeValues = make([]big.Int, len(params.LeafHigherRangeValues))
	for i := 0; i < len(params.LeafHigherRangeValues); i++ {
		err = fromHex(&p.LeafHigherRangeValues[i], params.LeafHigherRangeValues[i])
		if err != nil {
			return err
		}
	}

	p.LeafIndices = make([]uint32, len(params.LeafIndices))
	for i := 0; i < len(params.LeafIndices); i++ {
		p.LeafIndices[i] = params.LeafIndices[i]
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
