package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type NonInclusionParametersJSON struct {
	Root           []string   `json:"root"`
	InPathIndices  []uint32   `json:"inPathIndices"`
	InPathElements [][]string `json:"inPathElements"`
	Value          []string   `json:"value"`

	LeafLowerRangeValue  []string `json:"leafLowerRangeValue"`
	LeafHigherRangeValue []string `json:"leafHigherRangeValue"`
	LeafIndex            []uint32 `json:"leafIndex"`
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

	paramsJson.Root = make([]string, len(p.Root))
	for i := 0; i < len(p.Root); i++ {
		paramsJson.Root[i] = toHex(&p.Root[i])
	}

	paramsJson.Value = make([]string, len(p.Value))
	for i := 0; i < len(p.Value); i++ {
		paramsJson.Value[i] = toHex(&p.Value[i])
	}

	paramsJson.LeafLowerRangeValue = make([]string, len(p.LeafLowerRangeValue))
	for i := 0; i < len(p.LeafLowerRangeValue); i++ {
		paramsJson.LeafLowerRangeValue[i] = toHex(&p.LeafLowerRangeValue[i])
	}

	paramsJson.LeafHigherRangeValue = make([]string, len(p.LeafHigherRangeValue))
	for i := 0; i < len(p.LeafHigherRangeValue); i++ {
		paramsJson.LeafHigherRangeValue[i] = toHex(&p.LeafHigherRangeValue[i])
	}

	paramsJson.LeafIndex = make([]uint32, len(p.LeafIndex))
	for i := 0; i < len(p.LeafIndex); i++ {
		paramsJson.LeafIndex[i] = p.LeafIndex[i]
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

	p.Root = make([]big.Int, len(params.Root))
	for i := 0; i < len(params.Root); i++ {
		err = fromHex(&p.Root[i], params.Root[i])
		if err != nil {
			return err
		}
	}

	p.Value = make([]big.Int, len(params.Value))
	for i := 0; i < len(params.Value); i++ {
		err = fromHex(&p.Value[i], params.Value[i])
		if err != nil {
			return err
		}
	}

	p.LeafLowerRangeValue = make([]big.Int, len(params.LeafLowerRangeValue))
	for i := 0; i < len(params.LeafLowerRangeValue); i++ {
		err = fromHex(&p.LeafLowerRangeValue[i], params.LeafLowerRangeValue[i])
		if err != nil {
			return err
		}
	}

	p.LeafHigherRangeValue = make([]big.Int, len(params.LeafHigherRangeValue))
	for i := 0; i < len(params.LeafHigherRangeValue); i++ {
		err = fromHex(&p.LeafHigherRangeValue[i], params.LeafHigherRangeValue[i])
		if err != nil {
			return err
		}
	}

	p.LeafIndex = make([]uint32, len(params.LeafIndex))
	for i := 0; i < len(params.LeafIndex); i++ {
		p.LeafIndex[i] = params.LeafIndex[i]
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
