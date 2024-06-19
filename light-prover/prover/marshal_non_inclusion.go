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
	NextIndex            uint32   `json:"nextIndex"`
}

type NonInclusionParametersJSON struct {
	Inputs []NonInclusionProofInputsJSON `json:"new-addresses"`
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
	paramsJson.Inputs = make([]NonInclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.Inputs[i].Root = toHex(&p.Inputs[i].Root)
		paramsJson.Inputs[i].Value = toHex(&p.Inputs[i].Value)
		paramsJson.Inputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.Inputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.Inputs[i].PathElements[j] = toHex(&p.Inputs[i].PathElements[j])
		}
		paramsJson.Inputs[i].LeafLowerRangeValue = toHex(&p.Inputs[i].LeafLowerRangeValue)
		paramsJson.Inputs[i].LeafHigherRangeValue = toHex(&p.Inputs[i].LeafHigherRangeValue)
		paramsJson.Inputs[i].NextIndex = p.Inputs[i].NextIndex
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
