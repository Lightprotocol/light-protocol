package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type NonInclusionProofInputsJSON struct {
	Root                 string   `json:"root"`
	Value                string   `json:"value"`
	PathIndex            uint32   `json:"pathIndex"`
	PathElements         []string `json:"pathElements"`
	LeafLowerRangeValue  string   `json:"leafLowerRangeValue"`
	LeafHigherRangeValue string   `json:"leafHigherRangeValue"`
}

type NonInclusionParametersJSON struct {
	CircuitType        CircuitType                   `json:"circuitType"`
	AddressTreeHeight  uint32                        `json:"addressTreeHeight"`
	PublicInputHash    string                        `json:"publicInputHash"`
	NonInclusionInputs []NonInclusionProofInputsJSON `json:"newAddresses"`
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
	paramsJson := p.CreateNonInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *NonInclusionParameters) CreateNonInclusionParametersJSON() NonInclusionParametersJSON {
	paramsJson := NonInclusionParametersJSON{}
	paramsJson.CircuitType = NonInclusionCircuitType
	paramsJson.AddressTreeHeight = uint32(len(p.Inputs[0].PathElements))
	paramsJson.NonInclusionInputs = make([]NonInclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.NonInclusionInputs[i].Root = toHex(&p.Inputs[i].Root)
		paramsJson.NonInclusionInputs[i].Value = toHex(&p.Inputs[i].Value)
		paramsJson.NonInclusionInputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.NonInclusionInputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.NonInclusionInputs[i].PathElements[j] = toHex(&p.Inputs[i].PathElements[j])
		}
		paramsJson.NonInclusionInputs[i].LeafLowerRangeValue = toHex(&p.Inputs[i].LeafLowerRangeValue)
		paramsJson.NonInclusionInputs[i].LeafHigherRangeValue = toHex(&p.Inputs[i].LeafHigherRangeValue)
	}
	paramsJson.PublicInputHash = toHex(&p.PublicInputHash)
	paramsJson.CircuitType = NonInclusionCircuitType
	return paramsJson
}

func (p *NonInclusionParameters) UnmarshalJSON(data []byte) error {
	var params NonInclusionParametersJSON
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

func (p *NonInclusionParameters) UpdateWithJSON(params NonInclusionParametersJSON, err error) error {
	fromHex(&p.PublicInputHash, params.PublicInputHash)
	p.Inputs = make([]NonInclusionInputs, len(params.NonInclusionInputs))
	for i := 0; i < len(params.NonInclusionInputs); i++ {
		err = fromHex(&p.Inputs[i].Root, params.NonInclusionInputs[i].Root)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].Value, params.NonInclusionInputs[i].Value)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.NonInclusionInputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.NonInclusionInputs[i].PathElements))
		for j := 0; j < len(params.NonInclusionInputs[i].PathElements); j++ {
			err = fromHex(&p.Inputs[i].PathElements[j], params.NonInclusionInputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
		err = fromHex(&p.Inputs[i].LeafLowerRangeValue, params.NonInclusionInputs[i].LeafLowerRangeValue)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].LeafHigherRangeValue, params.NonInclusionInputs[i].LeafHigherRangeValue)
		if err != nil {
			return err
		}
	}
	return nil
}
