package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type V1NonInclusionProofInputsJSON struct {
	Root                 string   `json:"root"`
	Value                string   `json:"value"`
	PathIndex            uint32   `json:"pathIndex"`
	PathElements         []string `json:"pathElements"`
	LeafLowerRangeValue  string   `json:"leafLowerRangeValue"`
	LeafHigherRangeValue string   `json:"leafHigherRangeValue"`
	NextIndex            uint32   `json:"nextIndex"`
}

type V1NonInclusionParametersJSON struct {
	CircuitType       CircuitType                         `json:"circuitType"`
	AddressTreeHeight uint32                              `json:"addressTreeHeight"`
	Inputs            []V1NonInclusionProofInputsJSON `json:"newAddresses"`
}

func V1ParseNonInclusion(inputJSON string) (V1NonInclusionParameters, error) {
	var proofData V1NonInclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return V1NonInclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *V1NonInclusionParameters) V1MarshalJSON() ([]byte, error) {
	paramsJson := p.V1CreateNonInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *V1NonInclusionParameters) V1CreateNonInclusionParametersJSON() V1NonInclusionParametersJSON {
	paramsJson := V1NonInclusionParametersJSON{}
	paramsJson.Inputs = make([]V1NonInclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.Inputs[i].Root = toHex(&p.Inputs[i].Root)
		paramsJson.Inputs[i].Value = toHex(&p.Inputs[i].Value)
		paramsJson.Inputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.Inputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.Inputs[i].PathElements[j] = toHex(&p.Inputs[i].PathElements[j])
			paramsJson.Inputs[i].NextIndex = p.Inputs[i].NextIndex
		}
		paramsJson.Inputs[i].LeafLowerRangeValue = toHex(&p.Inputs[i].LeafLowerRangeValue)
		paramsJson.Inputs[i].LeafHigherRangeValue = toHex(&p.Inputs[i].LeafHigherRangeValue)
	}
	return paramsJson
}

func (p *V1NonInclusionParameters) UnmarshalJSON(data []byte) error {
	var params V1NonInclusionParametersJSON
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

func (p *V1NonInclusionParameters) UpdateWithJSON(params V1NonInclusionParametersJSON, err error) error {
	p.Inputs = make([]V1NonInclusionInputs, len(params.Inputs))
	for i := 0; i < len(params.Inputs); i++ {
		err = fromHex(&p.Inputs[i].Root, params.Inputs[i].Root)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].Value, params.Inputs[i].Value)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.Inputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.Inputs[i].PathElements))
		for j := 0; j < len(params.Inputs[i].PathElements); j++ {
			err = fromHex(&p.Inputs[i].PathElements[j], params.Inputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
		err = fromHex(&p.Inputs[i].LeafLowerRangeValue, params.Inputs[i].LeafLowerRangeValue)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].LeafHigherRangeValue, params.Inputs[i].LeafHigherRangeValue)
		if err != nil {
			return err
		}
		p.Inputs[i].NextIndex = params.Inputs[i].NextIndex
	}
	return nil
}
