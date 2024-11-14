package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type BatchAddressTreeAppendParametersJSON struct {
	PublicInputHash       string     `json:"publicInputHash"`
	OldRoot               string     `json:"oldRoot"`
	NewRoot               string     `json:"newRoot"`
	HashchainHash         string     `json:"hashchainHash"`
	StartIndex            uint32     `json:"startIndex"`
	LowElementValues      []string   `json:"lowElementValues"`
	LowElementIndices     []string   `json:"lowElementIndices"`
	LowElementNextIndices []string   `json:"lowElementNextIndices"`
	LowElementNextValues  []string   `json:"lowElementNextValues"`
	NewElementValues      []string   `json:"newElementValues"`
	LowElementProofs      [][]string `json:"lowElementProofs"`
	NewElementProofs      [][]string `json:"newElementProofs"`
	TreeHeight            uint32     `json:"treeHeight"`
	BatchSize             uint32     `json:"batchSize"`
}

func ParseBatchAddressTreeInput(inputJSON string) (BatchAddressTreeAppendParameters, error) {
	var jsonParams BatchAddressTreeAppendParametersJSON
	err := json.Unmarshal([]byte(inputJSON), &jsonParams)
	if err != nil {
		return BatchAddressTreeAppendParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}

	var params BatchAddressTreeAppendParameters
	err = params.UpdateWithJSON(jsonParams)
	if err != nil {
		return BatchAddressTreeAppendParameters{}, err
	}

	return params, nil
}

func (p *BatchAddressTreeAppendParameters) UpdateWithJSON(jsonParams BatchAddressTreeAppendParametersJSON) error {
	var err error

	fromBase10(p.PublicInputHash, jsonParams.PublicInputHash)
	fromBase10(p.OldRoot, jsonParams.OldRoot)

	fromBase10(p.NewRoot, jsonParams.NewRoot)

	fromBase10(p.HashchainHash, jsonParams.HashchainHash)

	p.StartIndex = jsonParams.StartIndex
	p.TreeHeight = jsonParams.TreeHeight
	p.BatchSize = jsonParams.BatchSize

	p.LowElementValues, err = convertStringSliceToBigIntSlice(jsonParams.LowElementValues)
	if err != nil {
		return err
	}
	p.LowElementIndices, err = convertStringSliceToBigIntSlice(jsonParams.LowElementIndices)
	if err != nil {
		return err
	}
	p.LowElementNextIndices, err = convertStringSliceToBigIntSlice(jsonParams.LowElementNextIndices)
	if err != nil {
		return err
	}
	p.LowElementNextValues, err = convertStringSliceToBigIntSlice(jsonParams.LowElementNextValues)
	if err != nil {
		return err
	}
	p.NewElementValues, err = convertStringSliceToBigIntSlice(jsonParams.NewElementValues)
	if err != nil {
		return err
	}

	p.LowElementProofs, err = convertNestedStringSliceToBigIntSlice(jsonParams.LowElementProofs)
	if err != nil {
		return err
	}
	p.NewElementProofs, err = convertNestedStringSliceToBigIntSlice(jsonParams.NewElementProofs)
	if err != nil {
		return err
	}

	return nil
}

func convertStringSliceToBigIntSlice(stringSlice []string) ([]big.Int, error) {
	result := make([]big.Int, len(stringSlice))
	for i, s := range stringSlice {
		p := new(big.Int)

		fromBase10(p, s)
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

			fromBase10(p, s)

			innerResult[j] = *p
		}
		result[i] = innerResult
	}
	return result, nil
}

func (p *BatchAddressTreeAppendParameters) UnmarshalJSON(data []byte) error {
	var jsonParams BatchAddressTreeAppendParametersJSON
	err := json.Unmarshal(data, &jsonParams)

	err = p.UpdateWithJSON(jsonParams)
	return err
}
