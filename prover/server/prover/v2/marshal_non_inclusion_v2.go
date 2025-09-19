package v2

import (
	"encoding/json"
	"fmt"
	"light/light-prover/prover/common"
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

type V2NonInclusionParametersJSON struct {
	CircuitType        common.CircuitType            `json:"circuitType"`
	AddressTreeHeight  uint32                        `json:"addressTreeHeight"`
	PublicInputHash    string                        `json:"publicInputHash"`
	NonInclusionInputs []NonInclusionProofInputsJSON `json:"newAddresses"`
}

func ParseNonInclusion(inputJSON string) (V2NonInclusionParameters, error) {
	var proofData V2NonInclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return V2NonInclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *V2NonInclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateV2NonInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *V2NonInclusionParameters) CreateV2NonInclusionParametersJSON() V2NonInclusionParametersJSON {
	paramsJson := V2NonInclusionParametersJSON{}
	paramsJson.CircuitType = common.NonInclusionCircuitType
	paramsJson.AddressTreeHeight = uint32(len(p.Inputs[0].PathElements))
	paramsJson.NonInclusionInputs = make([]NonInclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.NonInclusionInputs[i].Root = common.ToHex(&p.Inputs[i].Root)
		paramsJson.NonInclusionInputs[i].Value = common.ToHex(&p.Inputs[i].Value)
		paramsJson.NonInclusionInputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.NonInclusionInputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.NonInclusionInputs[i].PathElements[j] = common.ToHex(&p.Inputs[i].PathElements[j])
		}
		paramsJson.NonInclusionInputs[i].LeafLowerRangeValue = common.ToHex(&p.Inputs[i].LeafLowerRangeValue)
		paramsJson.NonInclusionInputs[i].LeafHigherRangeValue = common.ToHex(&p.Inputs[i].LeafHigherRangeValue)
	}
	paramsJson.PublicInputHash = common.ToHex(&p.PublicInputHash)
	paramsJson.CircuitType = common.NonInclusionCircuitType
	return paramsJson
}

func (p *V2NonInclusionParameters) UnmarshalJSON(data []byte) error {
	var params V2NonInclusionParametersJSON
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

func (p *V2NonInclusionParameters) UpdateWithJSON(params V2NonInclusionParametersJSON, err error) error {
	common.FromHex(&p.PublicInputHash, params.PublicInputHash)
	p.Inputs = make([]NonInclusionInputs, len(params.NonInclusionInputs))
	for i := 0; i < len(params.NonInclusionInputs); i++ {
		err = common.FromHex(&p.Inputs[i].Root, params.NonInclusionInputs[i].Root)
		if err != nil {
			return err
		}
		err = common.FromHex(&p.Inputs[i].Value, params.NonInclusionInputs[i].Value)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.NonInclusionInputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.NonInclusionInputs[i].PathElements))
		for j := 0; j < len(params.NonInclusionInputs[i].PathElements); j++ {
			err = common.FromHex(&p.Inputs[i].PathElements[j], params.NonInclusionInputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
		err = common.FromHex(&p.Inputs[i].LeafLowerRangeValue, params.NonInclusionInputs[i].LeafLowerRangeValue)
		if err != nil {
			return err
		}
		err = common.FromHex(&p.Inputs[i].LeafHigherRangeValue, params.NonInclusionInputs[i].LeafHigherRangeValue)
		if err != nil {
			return err
		}
	}
	return nil
}
