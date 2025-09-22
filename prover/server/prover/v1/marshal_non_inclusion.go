package v1

import (
	"encoding/json"
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
	NextIndex            uint32   `json:"nextIndex"`
}

type NonInclusionParametersJSON struct {
	CircuitType       common.CircuitType            `json:"circuitType"`
	AddressTreeHeight uint32                        `json:"addressTreeHeight"`
	Inputs            []NonInclusionProofInputsJSON `json:"newAddresses"`
}

func (p *NonInclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateNonInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *NonInclusionParameters) CreateNonInclusionParametersJSON() NonInclusionParametersJSON {
	paramsJson := NonInclusionParametersJSON{}
	paramsJson.CircuitType = common.NonInclusionCircuitType
	paramsJson.AddressTreeHeight = 26 // v1 always uses height 26
	paramsJson.Inputs = make([]NonInclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.Inputs[i].Root = common.ToHex(&p.Inputs[i].Root)
		paramsJson.Inputs[i].Value = common.ToHex(&p.Inputs[i].Value)
		paramsJson.Inputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.Inputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.Inputs[i].PathElements[j] = common.ToHex(&p.Inputs[i].PathElements[j])
			paramsJson.Inputs[i].NextIndex = p.Inputs[i].NextIndex
		}
		paramsJson.Inputs[i].LeafLowerRangeValue = common.ToHex(&p.Inputs[i].LeafLowerRangeValue)
		paramsJson.Inputs[i].LeafHigherRangeValue = common.ToHex(&p.Inputs[i].LeafHigherRangeValue)
	}
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
	p.Inputs = make([]NonInclusionInputs, len(params.Inputs))
	for i := 0; i < len(params.Inputs); i++ {
		err = common.FromHex(&p.Inputs[i].Root, params.Inputs[i].Root)
		if err != nil {
			return err
		}
		err = common.FromHex(&p.Inputs[i].Value, params.Inputs[i].Value)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.Inputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.Inputs[i].PathElements))
		for j := 0; j < len(params.Inputs[i].PathElements); j++ {
			err = common.FromHex(&p.Inputs[i].PathElements[j], params.Inputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
		err = common.FromHex(&p.Inputs[i].LeafLowerRangeValue, params.Inputs[i].LeafLowerRangeValue)
		if err != nil {
			return err
		}
		err = common.FromHex(&p.Inputs[i].LeafHigherRangeValue, params.Inputs[i].LeafHigherRangeValue)
		if err != nil {
			return err
		}
		p.Inputs[i].NextIndex = params.Inputs[i].NextIndex
	}
	return nil
}
