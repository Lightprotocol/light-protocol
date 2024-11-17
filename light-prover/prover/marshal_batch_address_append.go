package prover

import (
	"encoding/json"
	"math/big"
)

type BatchAddressAppendParametersJSON struct {
	PublicInputHash       string     `json:"PublicInputHash"`
	OldRoot               string     `json:"OldRoot"`
	NewRoot               string     `json:"NewRoot"`
	HashchainHash         string     `json:"HashchainHash"`
	StartIndex            uint32     `json:"StartIndex"`
	LowElementValues      []string   `json:"LowElementValues"`
	LowElementIndices     []string   `json:"LowElementIndices"`
	LowElementNextIndices []string   `json:"LowElementNextIndices"`
	LowElementNextValues  []string   `json:"LowElementNextValues"`
	NewElementValues      []string   `json:"NewElementValues"`
	LowElementProofs      [][]string `json:"LowElementProofs"`
	NewElementProofs      [][]string `json:"NewElementProofs"`
	TreeHeight            uint32     `json:"TreeHeight"`
	BatchSize             uint32     `json:"BatchSize"`
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
	paramsJson.PublicInputHash = toHex(p.PublicInputHash)
	paramsJson.OldRoot = toHex(p.OldRoot)
	paramsJson.NewRoot = toHex(p.NewRoot)
	paramsJson.HashchainHash = toHex(p.HashchainHash)
	paramsJson.StartIndex = p.StartIndex
	paramsJson.TreeHeight = p.TreeHeight
	paramsJson.BatchSize = p.BatchSize

	paramsJson.LowElementProofs = make([][]string, len(p.LowElementProofs))
	for i := 0; i < len(p.LowElementProofs); i++ {
		paramsJson.LowElementProofs[i] = make([]string, len(p.LowElementProofs[i]))
		for j := 0; j < len(p.LowElementProofs[i]); j++ {
			paramsJson.LowElementProofs[i][j] = toHex(&p.LowElementProofs[i][j])
		}
	}

	paramsJson.NewElementProofs = make([][]string, len(p.NewElementProofs))
	for i := 0; i < len(p.NewElementProofs); i++ {
		paramsJson.NewElementProofs[i] = make([]string, len(p.NewElementProofs[i]))
		for j := 0; j < len(p.NewElementProofs[i]); j++ {
			paramsJson.NewElementProofs[i][j] = toHex(&p.NewElementProofs[i][j])
		}
	}

	paramsJson.LowElementValues = make([]string, len(p.LowElementValues))
	for i := 0; i < len(p.LowElementValues); i++ {
		paramsJson.LowElementValues[i] = toHex(&p.LowElementValues[i])
	}

	paramsJson.LowElementIndices = make([]string, len(p.LowElementIndices))
	for i := 0; i < len(p.LowElementIndices); i++ {
		paramsJson.LowElementIndices[i] = toHex(&p.LowElementIndices[i])
	}

	paramsJson.LowElementNextIndices = make([]string, len(p.LowElementNextIndices))
	for i := 0; i < len(p.LowElementNextIndices); i++ {
		paramsJson.LowElementNextIndices[i] = toHex(&p.LowElementNextIndices[i])
	}

	paramsJson.LowElementNextValues = make([]string, len(p.LowElementNextValues))
	for i := 0; i < len(p.LowElementNextValues); i++ {
		paramsJson.LowElementNextValues[i] = toHex(&p.LowElementNextValues[i])
	}

	paramsJson.NewElementValues = make([]string, len(p.NewElementValues))
	for i := 0; i < len(p.NewElementValues); i++ {
		paramsJson.NewElementValues[i] = toHex(&p.NewElementValues[i])
	}

	return paramsJson
}

func (p *BatchAddressAppendParameters) UpdateWithJSON(params BatchAddressAppendParametersJSON) error {
	var err error
	p.TreeHeight = params.TreeHeight
	p.BatchSize = params.BatchSize
	p.StartIndex = params.StartIndex

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

	p.HashchainHash = new(big.Int)
	err = fromHex(p.HashchainHash, params.HashchainHash)
	if err != nil {
		return err
	}

	p.PublicInputHash = new(big.Int)
	err = fromHex(p.PublicInputHash, params.PublicInputHash)
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
	p.LowElementNextIndices, err = convertStringSliceToBigIntSlice(params.LowElementNextIndices)
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

		fromHex(p, s)
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

			fromHex(p, s)

			innerResult[j] = *p
		}
		result[i] = innerResult
	}
	return result, nil
}
