package v2

import (
	"encoding/json"
	"light/light-prover/prover/common"
	"math/big"
)

type BatchAddressAppendParametersJSON struct {
	CircuitType          common.CircuitType `json:"circuitType"`
	StateTreeHeight      uint32             `json:"stateTreeHeight"`
	PublicInputHash      string             `json:"publicInputHash"`
	OldRoot              string             `json:"oldRoot"`
	NewRoot              string             `json:"newRoot"`
	HashchainHash        string             `json:"hashchainHash"`
	StartIndex           uint64             `json:"startIndex"`
	LowElementValues     []string           `json:"lowElementValues"`
	LowElementIndices    []string           `json:"lowElementIndices"`
	LowElementNextValues []string           `json:"lowElementNextValues"`
	NewElementValues     []string           `json:"newElementValues"`
	LowElementProofs     [][]string         `json:"lowElementProofs"`
	NewElementProofs     [][]string         `json:"newElementProofs"`
	TreeHeight           uint32             `json:"treeHeight"`
	BatchSize            uint32             `json:"batchSize"`
}

func ParseBatchAddressAppendInput(inputJSON string) (BatchAddressAppendParameters, error) {
	var params BatchAddressAppendParameters
	err := json.Unmarshal([]byte(inputJSON), &params)
	if err != nil {
		return BatchAddressAppendParameters{}, err
	}

	return params, nil
}

func (p *BatchAddressAppendParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateBatchAddressAppendParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *BatchAddressAppendParameters) UnmarshalJSON(data []byte) error {
	var params BatchAddressAppendParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	return p.UpdateWithJSON(params)
}

func (p *BatchAddressAppendParameters) CreateBatchAddressAppendParametersJSON() BatchAddressAppendParametersJSON {
	paramsJson := BatchAddressAppendParametersJSON{}
	paramsJson.CircuitType = common.BatchAddressAppendCircuitType
	paramsJson.PublicInputHash = common.ToHex(p.PublicInputHash)
	paramsJson.OldRoot = common.ToHex(p.OldRoot)
	paramsJson.NewRoot = common.ToHex(p.NewRoot)
	paramsJson.HashchainHash = common.ToHex(p.HashchainHash)
	paramsJson.StartIndex = p.StartIndex
	paramsJson.TreeHeight = p.TreeHeight
	paramsJson.BatchSize = p.BatchSize

	paramsJson.LowElementProofs = make([][]string, len(p.LowElementProofs))
	for i := 0; i < len(p.LowElementProofs); i++ {
		paramsJson.LowElementProofs[i] = make([]string, len(p.LowElementProofs[i]))
		for j := 0; j < len(p.LowElementProofs[i]); j++ {
			paramsJson.LowElementProofs[i][j] = common.ToHex(&p.LowElementProofs[i][j])
		}
	}

	paramsJson.NewElementProofs = make([][]string, len(p.NewElementProofs))
	for i := 0; i < len(p.NewElementProofs); i++ {
		paramsJson.NewElementProofs[i] = make([]string, len(p.NewElementProofs[i]))
		for j := 0; j < len(p.NewElementProofs[i]); j++ {
			paramsJson.NewElementProofs[i][j] = common.ToHex(&p.NewElementProofs[i][j])
		}
	}

	paramsJson.LowElementValues = make([]string, len(p.LowElementValues))
	for i := 0; i < len(p.LowElementValues); i++ {
		paramsJson.LowElementValues[i] = common.ToHex(&p.LowElementValues[i])
	}

	paramsJson.LowElementIndices = make([]string, len(p.LowElementIndices))
	for i := 0; i < len(p.LowElementIndices); i++ {
		paramsJson.LowElementIndices[i] = common.ToHex(&p.LowElementIndices[i])
	}

	paramsJson.LowElementNextValues = make([]string, len(p.LowElementNextValues))
	for i := 0; i < len(p.LowElementNextValues); i++ {
		paramsJson.LowElementNextValues[i] = common.ToHex(&p.LowElementNextValues[i])
	}

	paramsJson.NewElementValues = make([]string, len(p.NewElementValues))
	for i := 0; i < len(p.NewElementValues); i++ {
		paramsJson.NewElementValues[i] = common.ToHex(&p.NewElementValues[i])
	}

	return paramsJson
}

func (p *BatchAddressAppendParameters) UpdateWithJSON(params BatchAddressAppendParametersJSON) error {
	var err error
	p.TreeHeight = params.TreeHeight
	p.BatchSize = params.BatchSize
	p.StartIndex = params.StartIndex

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

	p.HashchainHash = new(big.Int)
	err = common.FromHex(p.HashchainHash, params.HashchainHash)
	if err != nil {
		return err
	}

	p.PublicInputHash = new(big.Int)
	err = common.FromHex(p.PublicInputHash, params.PublicInputHash)
	if err != nil {
		return err
	}

	p.LowElementValues, err = convertStringSliceToBigIntSlice(params.LowElementValues)
	if err != nil {
		return err
	}
	p.LowElementIndices, err = convertStringSliceToBigIntSlice(params.LowElementIndices)
	if err != nil {
		return err
	}
	p.LowElementNextValues, err = convertStringSliceToBigIntSlice(params.LowElementNextValues)
	if err != nil {
		return err
	}
	p.NewElementValues, err = convertStringSliceToBigIntSlice(params.NewElementValues)
	if err != nil {
		return err
	}

	p.LowElementProofs, err = convertNestedStringSliceToBigIntSlice(params.LowElementProofs)
	if err != nil {
		return err
	}
	p.NewElementProofs, err = convertNestedStringSliceToBigIntSlice(params.NewElementProofs)
	if err != nil {
		return err
	}
	return nil
}

func convertStringSliceToBigIntSlice(stringSlice []string) ([]big.Int, error) {
	result := make([]big.Int, len(stringSlice))
	for i, s := range stringSlice {
		p := new(big.Int)

		common.FromHex(p, s)
		result[i] = *p
	}
	return result, nil
}

func convertNestedStringSliceToBigIntSlice(nestedStringSlice [][]string) ([][]big.Int, error) {
	result := make([][]big.Int, len(nestedStringSlice))
	for i, innerSlice := range nestedStringSlice {
		innerResult := make([]big.Int, len(innerSlice))
		for j, s := range innerSlice {
			p := new(big.Int)

			common.FromHex(p, s)

			innerResult[j] = *p
		}
		result[i] = innerResult
	}
	return result, nil
}
